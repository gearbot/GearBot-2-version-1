use std::sync::atomic::Ordering;
use std::sync::Arc;

use log::{debug, trace};
use twilight_gateway::Event;

use crate::core::BotContext;
use crate::error::EventHandlerError;
use crate::parser::Parser;

pub async fn handle_event<'a>(shard_id: u64, event: Event, ctx: Arc<BotContext>) -> Result<(), EventHandlerError> {
    match event {
        Event::MessageCreate(msg) if !msg.author.bot => {
            trace!("Received a message from {}, saying {}", msg.author.name, msg.content);

            let p = match msg.guild_id {
                Some(guild_id) => {
                    let guild = ctx.cache.get_guild(&guild_id);
                    match guild {
                        Some(g) => {
                            if !g.complete.load(Ordering::SeqCst) {
                                debug!("Message received in {} but the guild isn't fully cached yet!", g.id);
                                return Ok(()); //not cached yet, just ignore for now
                            }
                        }
                        None => return Ok(()), // we didn't even get a guild create yet
                    }

                    let config = ctx.get_config(guild_id).await?;
                    config.prefix.clone()
                }
                None => String::from("!"),
            };

            let prefix = if msg.content.starts_with(&p) {
                Some(p)
            } else {
                let mention_1 = format!("<@{}>", ctx.bot_user.id);
                let mention_2 = format!("<@!{}>", ctx.bot_user.id);
                if msg.content.starts_with(&mention_1) {
                    Some(mention_1)
                } else if msg.content.starts_with(&mention_2) {
                    Some(mention_2)
                } else {
                    None
                }
            };

            if let Some(prefix) = prefix {
                Parser::figure_it_out(&prefix, msg, ctx, shard_id).await?;
            }
        }
        Event::MessageUpdate(update) => {
            trace!("Message updated to {:?}", update.content);
        }
        _ => (),
    }

    Ok(())
}
