mod log_data;
mod log_filter;
mod log_type;

pub use log_data::LogData;
pub use log_filter::LogFilter;
pub use log_type::DataLessLogType;
pub use log_type::LogType;

use crate::core::guild_config::LogStyle;
use crate::core::BotContext;
use crate::gearbot_error;
use futures_util::SinkExt;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use twilight_model::id::{ChannelId, GuildId};

pub async fn run(ctx: Arc<BotContext>, mut receiver: UnboundedReceiver<LogData>) {
    log::info!("Logpump started!");
    let mut outputs: HashMap<ChannelId, UnboundedSender<Arc<LogType>>> = HashMap::new();
    loop {
        //it's impossible to drop the sender at this time
        let log = receiver.recv().await.unwrap();
        log::debug!("log data received: {:?}", log);
        let log_type = Arc::new(log.log_type);
        let guild_id = log.guild.clone();
        match ctx.get_config(guild_id).await {
            Ok(config) => {
                for (channel_id, log_config) in config.log_channels.iter() {
                    //check if it could go to this channel
                    if log_config.categories.contains(&log_type.get_category())
                        && !log_config.disabled_keys.contains(&log_type.dataless())
                    {
                        let mut matches = false;
                        for filter in &log_config.filters {
                            if filter.matches(&log_type.dataless(), &log.source_channel, &log.source_user) {
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
                            let mut owned_sender = sender.clone();
                            if let Err(e) = owned_sender.send(log_type.clone()) {
                                // fail, channel must have expired, setup a new one
                                let (sender, receiver) = unbounded_channel();
                                tokio::spawn(pump(ctx.clone(), receiver, log.guild.clone(), channel_id.clone()));
                                let _ = sender.send(log_type.clone());
                                outputs.insert(channel_id.clone(), sender);
                            }
                        } else {
                            //we do not, create a new one and send the log
                            let (sender, receiver) = unbounded_channel();
                            tokio::spawn(pump(ctx.clone(), receiver, guild_id, channel_id.clone()));
                            let _ = sender.send(log_type.clone());
                            outputs.insert(channel_id.clone(), sender);
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
    mut receiver: UnboundedReceiver<Arc<LogType>>,
    guild_id: GuildId,
    channel_id: ChannelId,
) {
    let mut batch_size = 20;
    let mut todo: Vec<Arc<LogType>> = vec![];
    loop {
        if todo.len() < batch_size {
            //refill pls
            todo.append(&mut receive_up_to(batch_size, &mut receiver).await);
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
                    let future = match channel_config.style {
                        LogStyle::Text => {
                            batch_size = 20;
                            let mut output = String::from("");
                            //keep grabbing items while we have some left
                            while let Some(item) = todo.first() {
                                let extra = item.to_text();
                                //only add to the output and remove from todo if it actually fits
                                if output.len() + extra.len() < 2000 {
                                    output += extra;
                                    output += "\n";
                                    todo.remove(0);
                                } else {
                                    //didn't fit, we're done here
                                    break;
                                }
                            }
                            // assembly done, pack it into the future
                            ctx.http.create_message(channel_id).content(output).unwrap()
                        }
                        LogStyle::Embed => {
                            // we can only process one at a time, at this time
                            //TODO: hook up webhooks to do 10 at a time
                            batch_size = 1;
                            let data = todo.remove(0);
                            match data.to_embed() {
                                Ok(embed) => {
                                    match ctx.http.create_message(channel_id).embed(embed) {
                                        Ok(future) => future,
                                        Err(e) => {
                                            gearbot_error!("Failed to create logging embed: {} (data: {:?})", e, data);
                                            // fall back to the text version
                                            ctx.http.create_message(channel_id).content(data.to_text()).unwrap()
                                        }
                                    }
                                }
                                Err(e) => {
                                    gearbot_error!("Failed to create logging embed: {} (data: {:?})", e, data);
                                    // fall back to the text version
                                    ctx.http.create_message(channel_id).content(data.to_text()).unwrap()
                                }
                            }
                        }
                    };
                    // let's hope the future isn't a huge disappointment
                    if let Err(e) = future.await {
                        gearbot_error!("Logpump failure: {}", e)
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
}

async fn receive_up_to(count: usize, receiver: &mut UnboundedReceiver<Arc<LogType>>) -> Vec<Arc<LogType>> {
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
