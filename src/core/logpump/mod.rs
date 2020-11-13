mod log_data;
mod log_filter;
mod log_type;

const GEARBOT_LOGO: &str = include_str!("../../../assets/logo");

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
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use twilight_http::Error;
use twilight_model::guild::Permissions;
use twilight_model::id::{ChannelId, GuildId, WebhookId};
use unic_langid::LanguageIdentifier;

pub async fn run(ctx: Arc<BotContext>, mut receiver: UnboundedReceiver<LogData>) {
    log::info!("Logpump started!");
    let mut outputs: HashMap<ChannelId, UnboundedSender<Arc<LogData>>> = HashMap::new();
    loop {
        //it's impossible to drop the sender at this time
        let log = Arc::new(receiver.recv().await.unwrap());
        log::debug!("log data received: {:?}", log);
        let guild_id = log.guild;
        match ctx.get_config(guild_id).await {
            Ok(config) => {
                for (channel_id, log_config) in &config.log_channels {
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
                        //see if we have a sender from last time
                        if let Some(sender) = outputs.get(channel_id) {
                            //sender found, try try to re-use it
                            let owned_sender = sender.clone();
                            if let Err(_) = owned_sender.send(log.clone()) {
                                // fail, channel must have expired, setup a new one
                                let (sender, receiver) = unbounded_channel();
                                tokio::spawn(pump(ctx.clone(), receiver, log.guild, *channel_id));
                                ctx.stats.logpump_stats.active_pumps.inc();
                                let _ = sender.send(log.clone());
                                outputs.insert(*channel_id, sender);
                            }
                        } else {
                            //we do not, create a new one and send the log
                            let (sender, receiver) = unbounded_channel();
                            tokio::spawn(pump(ctx.clone(), receiver, guild_id, *channel_id));
                            let _ = sender.send(log.clone());
                            outputs.insert(*channel_id, sender);
                        }
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
    mut receiver: UnboundedReceiver<Arc<LogData>>,
    guild_id: GuildId,
    channel_id: ChannelId,
) {
    let batch_size = 20;
    let mut todo: Vec<Arc<LogData>> = vec![];
    let mut leftover: Vec<String> = vec![];
    let mut webhook_info = None;
    'outer: loop {
        if todo.len() < batch_size {
            //refill pls
            let mut received = receive_up_to(batch_size, &mut receiver).await;
            ctx.stats.logpump_stats.pending_logs.sub(received.len() as i64);
            todo.append(&mut received);
            if todo.is_empty() {
                // all out of refills, time to self-destruct
                return;
            }
        }
        // re-fetch config on each iteration in case it updated
        //returns the sending future for central handling of errors
        match ctx.get_config(guild_id).await {
            Ok(config) => {
                if let Some(channel_config) = config.log_channels.get(&channel_id) {
                    //config found, validate we have the correct permissions
                    //we are not checking for embed permissions since this will be handled by a webhook later

                    //TODO: alert if we are missing permissions?

                    let mut style = Some(channel_config.style.clone());
                    while let Some(s) = &style {
                        match can_send(&ctx, &channel_id, s, &mut webhook_info).await {
                            Ok(ok) => {
                                if ok {
                                    break;
                                } else {
                                    style = s.get_fallback();
                                }
                            }
                            Err(e) => {
                                gearbot_error!("Failed to determine if we can log or not, did the database die? Falling back just in case. {}", e);
                                style = s.get_fallback();
                            }
                        }
                    }

                    if let Some(style) = &style {
                        if let Err(e) = send(
                            &ctx,
                            &mut todo,
                            &mut leftover,
                            style,
                            &config.language,
                            channel_id,
                            &mut webhook_info,
                            channel_config.timestamps,
                        )
                        .await
                        {
                            gearbot_error!("Logpump failure: {}", e)
                        }
                    } else {
                        //we can't log anything, hit the self-destruct
                        break 'outer;
                    }
                } else {
                    log::warn!(
                        "Channel {} (in guild {}) was removed as log channel but something still tried to log to it!",
                        channel_id,
                        guild_id
                    )
                }
            }
            Err(e) => gearbot_error!("Failed to retrieve guild config {}: {}", guild_id, e),
        }
    }
    receiver.close();
    let mut count = 0;
    while receiver.try_recv().is_ok() {
        count += 1;
    }
    ctx.stats.logpump_stats.pending_logs.sub(count);
    ctx.stats.logpump_stats.active_pumps.dec();
}

async fn can_send(
    ctx: &Arc<BotContext>,
    channel_id: &ChannelId,
    style: &LogStyle,
    webhook_info: &mut Option<(WebhookId, String)>,
) -> Result<bool, OtherFailure> {
    let ok = match style {
        LogStyle::Text => ctx
            .get_channel_permissions_for(ctx.bot_user.id, *channel_id)
            .contains(Permissions::SEND_MESSAGES),
        LogStyle::Embed => {
            if webhook_info.is_none() {
                //do we have one in the database?
                *webhook_info = get_webhook(ctx, channel_id).await?;
            }
            webhook_info.is_some()
        }
    };
    Ok(ok)
}

async fn get_webhook(
    ctx: &Arc<BotContext>,
    channel_id: &ChannelId,
) -> Result<Option<(WebhookId, String)>, OtherFailure> {
    let mut webhook_info = ctx.datastore.get_webhook_parts(*channel_id).await?;
    if webhook_info.is_none() {
        //nope, can we make one?
        if ctx
            .get_channel_permissions_for(ctx.bot_user.id, *channel_id)
            .contains(Permissions::MANAGE_WEBHOOKS)
        {
            let webhook = ctx
                .http
                .create_webhook(*channel_id, "GearBot moderation logs")
                .avatar(GEARBOT_LOGO)
                .await?;
            let token = webhook.token.unwrap();
            ctx.datastore
                .insert_webhook(*channel_id, webhook.id, token.clone())
                .await?;
            webhook_info = Some((webhook.id, token))
        }
    }
    Ok(webhook_info)
}

async fn send(
    ctx: &Arc<BotContext>,
    todo: &mut Vec<Arc<LogData>>,
    left_over: &mut Vec<String>,
    log_style: &LogStyle,
    language: &LanguageIdentifier,
    channel_id: ChannelId,
    webhook_info: &mut Option<(WebhookId, String)>,
    timestamp: bool,
) -> Result<(), twilight_http::Error> {
    match log_style {
        LogStyle::Text => {
            let mut output = String::from("");
            //grab leftovers from last iteration
            while let Some(item) = left_over.first() {
                if output.len() + item.len() < 2000 {
                    output += item;
                    output += "\n";
                    left_over.remove(0);
                } else {
                    break;
                }
            }

            //keep grabbing items while we have some left

            while let Some(item) = todo.first() {
                // get the user responsible
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
                    String::from("")
                };

                let mut extra = format!(
                    "{} {} {}",
                    timestamp,
                    item.log_type.emoji().for_chat(),
                    item.log_type.to_text(&ctx, language, &user, &item.source_channel)
                );
                extra.truncate(2000);
                //only add to the output and remove from todo if it actually fits
                if output.len() + extra.len() < 2000 {
                    output += &extra;
                    output += "\n";
                    todo.remove(0);
                } else {
                    //didn't fit, we're done here
                    break;
                }
            }
            // assembly done, pack it into the future
            ctx.http.create_message(channel_id).content(output).unwrap().await?;
        }
        LogStyle::Embed => {
            let mut out = vec![];
            while let Some(_) = todo.first() {
                let data = todo.remove(0);
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
            let (webhook_id, token) = webhook_info.as_ref().unwrap();
            if let Err(e) = ctx.http.execute_webhook(webhook_id.clone(), token).embeds(out).await {
                match e {
                    Error::Response { status, .. } => {
                        if status == StatusCode::NOT_FOUND {
                            //webhook is gone, remove it
                            *webhook_info = None;
                            if let Err(e) = ctx.datastore.remove_webhook(channel_id).await {
                                gearbot_error!("Failed to remove webhook {} from the database: {}", channel_id, e)
                            }
                        }
                    }
                    _ => gearbot_error!("Logpump failure: {}", e),
                }
            }
        }
    }
    Ok(())
}

async fn receive_up_to(count: usize, receiver: &mut UnboundedReceiver<Arc<LogData>>) -> Vec<Arc<LogData>> {
    let mut out = vec![];
    if let Ok(log_data) = tokio::time::timeout(Duration::from_secs(6), receiver.recv()).await {
        //since we never drop the sender, we can never get a none value
        let log_data = log_data.unwrap();
        out.push(log_data);
        if count > 1 {
            while let Ok(data) = receiver.try_recv() {
                out.push(data);
                if out.len() >= count {
                    break;
                }
            }
        }
    }
    out
}
