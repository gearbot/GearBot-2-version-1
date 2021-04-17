mod log_data;
mod log_filter;
mod log_type;

const GEARBOT_LOGO: &str = include_str!("../../../assets/logo");
const GEARBOT_EMBED_SENDER: &str = "GearBot moderation logs";
const DISCORD_SIZE_LIMIT: usize = 2000;
const BATCH_SIZE: usize = 20;
const RECV_TIMEOUT: Duration = Duration::from_secs(4);

pub use log_data::LogData;
pub use log_filter::LogFilter;
pub use log_type::DataLessLogType;
pub use log_type::LogType;

use crate::core::bot_context::BotContext;
use crate::core::guild_config::LogStyle;
use crate::error::OtherFailure;
use crate::gearbot_error;

use hyper::StatusCode;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc::UnboundedReceiver, Mutex};
use tokio::time::timeout;
use twilight_http::Error;
use twilight_model::guild::Permissions;
use twilight_model::id::{ChannelId, GuildId, WebhookId};
use unic_langid::LanguageIdentifier;

type LogReceiver = UnboundedReceiver<Vec<LogData>>;
/// A time synchronization lock for a logging channel. Used to ensure
/// that messages always arrive in-order.
///
/// The `bool` represents the channel's validity for logging. Defaults to true.
///
/// A pump may change it to `false` to mark that a guild no longer needs this channel
/// to get logs anymore to avoid leaking memory.
type ChannelLock = Arc<Mutex<bool>>;

pub async fn run(ctx: Arc<BotContext>, mut top_receiver: UnboundedReceiver<LogData>) {
    log::info!("Logpump started!");
    let mut channel_sync_locks: HashMap<ChannelId, ChannelLock> = HashMap::new();
    loop {
        let mut to_send: Vec<Arc<LogData>> = Vec::with_capacity(BATCH_SIZE);

        // Sit and wait until we have something to do.
        let first_log = top_receiver.recv().await.unwrap();

        to_send.push(Arc::new(first_log));

        // If its a slow period, this will return early and give us whats around.
        receive_up_to(BATCH_SIZE, &mut top_receiver, &mut to_send).await;

        let log = to_send
            .first()
            .expect("bug: original log that started pup wasn't added to send list");

        log::debug!("log data received: {:?}", log);
        let guild_id = log.guild;
        match ctx.get_config(guild_id).await {
            Ok(config) => {
                for (channel_id, log_config) in &config.log_channels {
                    // Cheap clone since its just a bunch of `Arc`s inside.
                    let to_send = to_send.clone();
                    //check if it could go to this channel
                    if log_config.categories.contains(&log.log_type.get_category())
                        && !log_config.disabled_keys.contains(&log.log_type.dataless())
                    {
                        let mut matches = false;
                        for filter in &log_config.filters {
                            if filter.matches(&log.log_type.dataless(), &log.source_channel, &log.source_user) {
                                matches = true;
                                break;
                            }
                        }
                        if matches {
                            continue;
                        }

                        let channel_lock = Arc::clone(
                            &channel_sync_locks
                                .entry(*channel_id)
                                .or_insert(Arc::new(Mutex::new(true))),
                        );

                        if let Ok(lock) = channel_lock.try_lock() {
                            if *lock == false {
                                // A pump marked this channel as useless, so deallocate the lock since
                                // theres a chance we will never use it again.
                                channel_sync_locks.remove(channel_id);
                                continue;
                            }
                        }

                        tokio::spawn(pump(ctx.clone(), to_send, guild_id, *channel_id, channel_lock));
                    }
                }
            }
            Err(e) => gearbot_error!(
                "Logpump error: failed to retrieve config for guild {}: {}",
                log.guild,
                e
            ),
        }
    }
}

async fn pump(
    ctx: Arc<BotContext>,
    mut to_send: Vec<Arc<LogData>>,
    guild_id: GuildId,
    channel_id: ChannelId,
    channel_lock: ChannelLock,
) {
    // Ensure that only one task at a time can send logs for a channel.
    //
    // Humans are bad at piecing together out of order events, so this ensures that a specific channel gets all of its messages
    // delivered in order, between batches, regardless of how many pumps are currently active for a channel to scale across
    // a large number of incoming logs.
    let mut time_sync_barrier = channel_lock.lock().await;

    let log_count = to_send.len();
    ctx.stats.logpump_stats.active_pumps.inc();
    let mut webhook_info = None;
    'outer: loop {
        // Re-fetch config on each iteration in case it updated between long synchronization wait times
        match ctx.get_config(guild_id).await {
            Ok(config) => {
                // this is checked to not be empty.
                let channel_config = match config.log_channels.get(&channel_id) {
                    Some(c) => c,
                    None => {
                        log::warn!(
                            "Channel {} (in guild {}) was removed as log channel but something still tried to log to it!",
                            channel_id,
                            guild_id
                        );
                        // If the guild doesn't want this channel to get logs anymore, quit early and don't try sending them.
                        *time_sync_barrier = false;
                        break;
                    }
                };

                // config found, validate we have the correct permissions
                // we are not checking for embed permissions since this will be handled by a webhook later

                let mut style = Some(channel_config.style);

                // Worst case, if someones permissions are really messed up, they lose this log batch.
                if let Some(s) = &style {
                    match try_configure_to_send(&ctx, &channel_id, s, &mut webhook_info).await {
                        Ok(CanSend::MissingWebHook) => style = s.get_fallback(),
                        Ok(CanSend::MissingPermissions) => {
                            log::warn!("Missing permissions to log in channel {}, quitting pump", channel_id);
                            // If we can't log here, every attempt will fail anyway.
                            break;
                        }
                        Ok(CanSend::Yes) => {}
                        Err(e) => {
                            gearbot_error!("Failed to determine if we can log or not, did the database die? Falling back just in case. {}", e);
                            style = s.get_fallback();
                        }
                    }
                }

                let style = match &style {
                    Some(s) => s,
                    // we can't log anything, hit the self-destruct
                    None => break,
                };

                let send_style = match &webhook_info {
                    Some(info) => SendStyle::Webhook(info),
                    // We can't log anything here either.
                    //
                    // If the webhook gets trashed in the send loop and `try_configure_to_send` fails to get a new one,
                    // then we really cant do anything.
                    None if *style == LogStyle::Embed => {
                        gearbot_error!("Webhook information wasn't present with embed log styling");
                        break;
                    }
                    None => SendStyle::Channel, // We aren't using a webhook
                };

                while !to_send.is_empty() {
                    match send(
                        &ctx,
                        &mut to_send,
                        send_style,
                        &config.language,
                        channel_id,
                        channel_config.timestamps,
                    )
                    .await
                    {
                        Ok(None) => continue, // We weren't using a webhook.
                        Ok(Some(WebhookValidity::Valid)) => continue,
                        Ok(Some(WebhookValidity::Unusable)) => {
                            // The webhook isn't valid any longer, remove it.
                            //
                            // On the next iteration, try to get a new one or otherwise abort if nothing
                            // can be logged via webhook anymore.
                            webhook_info = None;
                            if let Err(e) = ctx.datastore.remove_webhook(channel_id).await {
                                gearbot_error!("Failed to remove webhook {} from the database: {}", channel_id, e);
                            }
                            // Break from the sending loop so we can try and get a new, valid, webhook.
                            continue 'outer;
                        }
                        Err(e) => gearbot_error!("Logpump failure: {}", e),
                    }
                }

                // We've finished sending the batch we got assigned, quit.
                break;
            }
            Err(e) => gearbot_error!("Failed to retrieve guild config {}: {}", guild_id, e),
        }
    }

    ctx.stats.logpump_stats.pending_logs.sub(log_count as i64);
    ctx.stats.logpump_stats.active_pumps.dec();
}

enum CanSend {
    MissingPermissions,
    MissingWebHook,
    Yes,
}

async fn try_configure_to_send(
    ctx: &Arc<BotContext>,
    channel_id: &ChannelId,
    style: &LogStyle,
    webhook_info: &mut Option<(WebhookId, String)>,
) -> Result<CanSend, OtherFailure> {
    match style {
        LogStyle::Text => {
            if ctx
                .get_channel_permissions_for(ctx.bot_user.id, *channel_id)
                .await
                .contains(Permissions::SEND_MESSAGES)
            {
                Ok(CanSend::Yes)
            } else {
                Ok(CanSend::MissingPermissions)
            }
        }
        LogStyle::Embed => {
            if webhook_info.is_none() {
                // Is there a webhook stored in the database?
                *webhook_info = get_webhook(ctx, channel_id).await?;
            }

            if webhook_info.is_some() {
                Ok(CanSend::Yes)
            } else {
                Ok(CanSend::MissingWebHook)
            }
        }
    }
}

async fn get_webhook(
    ctx: &Arc<BotContext>,
    channel_id: &ChannelId,
) -> Result<Option<(WebhookId, String)>, OtherFailure> {
    let webhook_info = match ctx.datastore.get_webhook_parts(*channel_id).await? {
        Some(hook) => Some(hook),
        None => {
            // Didn't have a webhook, see if we are allowed to create them.
            if ctx
                .get_channel_permissions_for(ctx.bot_user.id, *channel_id)
                .await
                .contains(Permissions::MANAGE_WEBHOOKS)
            {
                let webhook = ctx
                    .http
                    .create_webhook(*channel_id, GEARBOT_EMBED_SENDER)
                    .avatar(GEARBOT_LOGO)
                    .await?;

                let token = webhook.token.unwrap();
                ctx.datastore
                    .insert_webhook(*channel_id, webhook.id, token.clone())
                    .await?;

                Some((webhook.id, token))
            } else {
                None
            }
        }
    };

    Ok(webhook_info)
}

#[derive(Clone, Copy)]
enum SendStyle<'a> {
    Channel,
    Webhook(&'a (WebhookId, String)),
}

enum WebhookValidity {
    Valid,
    Unusable,
}

async fn send(
    ctx: &Arc<BotContext>,
    todo: &mut Vec<Arc<LogData>>,
    style: SendStyle<'_>,
    language: &LanguageIdentifier,
    channel_id: ChannelId,
    timestamp: bool,
) -> Result<Option<WebhookValidity>, twilight_http::Error> {
    match style {
        SendStyle::Channel => {
            let mut output = String::new();

            while let Some(item) = todo.first() {
                // Get the user responsible for causing the log event.
                let user = match ctx.get_user(item.source_user).await {
                    Ok(user) => user,
                    Err(e) => {
                        gearbot_error!("Failure retrieving user info for logging: {}", e);
                        log::error!("Log data: {:?}", item);
                        continue;
                    }
                };

                let timestamp = if timestamp {
                    format!("`[{}]`", chrono::Utc::now().format("%T").to_string())
                } else {
                    String::new()
                };

                let mut extra = format!(
                    "{} {} {}",
                    timestamp,
                    item.log_type.emoji().for_chat(),
                    item.log_type.to_text(&ctx, language, &user, &item.source_channel)
                );
                extra.truncate(DISCORD_SIZE_LIMIT);

                // Only add to the output and remove from todo if it actually fits
                if output.len() + extra.len() < DISCORD_SIZE_LIMIT {
                    output += &extra;
                    output += "\n";
                    todo.remove(0);
                } else {
                    // The message can't grow any longer without violating the size limit, time to send it.
                    break;
                }
            }

            // Assembly done, pack it into the future
            ctx.http.create_message(channel_id).content(output).unwrap().await?;
            Ok(None)
        }
        SendStyle::Webhook(webhook) => {
            let mut out = vec![];
            for data in todo.drain(..) {
                let user = match ctx.get_user(data.source_user).await {
                    Ok(user) => user,
                    Err(e) => {
                        gearbot_error!("Failure retrieving user info for logging: {}", e);
                        log::error!("Log data: {:?}", data);
                        continue;
                    }
                };

                match data.log_type.to_embed(&ctx, language, &user, &data.source_channel) {
                    Ok(embed) => {
                        out.push(embed);
                        if out.len() == 10 {
                            break;
                        }
                    }
                    Err(e) => {
                        gearbot_error!("Failed to create logging embed: {} (data: {:?})", e, data);
                    }
                }
            }

            let (webhook_id, token) = webhook;
            match ctx.http.execute_webhook(*webhook_id, token).embeds(out).await {
                Err(Error::Response { status, .. }) if status == StatusCode::NOT_FOUND => {
                    Ok(Some(WebhookValidity::Unusable))
                }
                Err(e) => Err(e),
                Ok(_) => Ok(Some(WebhookValidity::Valid)),
            }
        }
    }
}

async fn receive_up_to(count: usize, receiver: &mut UnboundedReceiver<LogData>, out: &mut Vec<Arc<LogData>>) {
    if let Ok(log_data) = timeout(RECV_TIMEOUT, receiver.recv()).await {
        // Since we never drop the sender, this can't fail.
        let log_data = log_data.unwrap();
        out.push(Arc::new(log_data));
        if count > 1 {
            while let Ok(Some(data)) = timeout(RECV_TIMEOUT, receiver.recv()).await {
                out.push(Arc::new(data));
                if out.len() >= count {
                    break;
                }
            }
        }
    }
}
