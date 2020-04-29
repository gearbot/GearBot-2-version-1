use std::sync::Arc;

use log::{debug, info};
use twilight::gateway::cluster::Event;

use crate::core::Context;
use crate::parser::Parser;
use crate::utils::Error;

pub async fn handle_event<'a>(event: Event, ctx: Arc<Context>) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) if !msg.author.bot => {
            if msg.author.bot {
                return Ok(());
            }
            info!(
                "Received a message from {}, saying {}",
                msg.author.name, msg.content
            );
            let p = match msg.guild_id {
                Some(guildid) => {
                    let config = ctx.get_config(msg.guild_id.unwrap().0 as i64).await?;
                    config.value().prefix.clone()
                }
                None => String::from("!"),
            };

            let prefix;
            if msg.content.starts_with(&p) {
                prefix = Some(p);
            } else if msg.content.starts_with(&format!("<@{}>", ctx.bot_user.id)) {
                prefix = Some(format!("<@{}>", ctx.bot_user.id))
            } else if msg.content.starts_with(&format!("<@!{}>", ctx.bot_user.id)) {
                prefix = Some(format!("<@!{}>", ctx.bot_user.id))
            } else {
                prefix = None;
            }

            if prefix.is_some() {
                Parser::figure_it_out(&prefix.unwrap(), msg, ctx).await?;
            }
        }
        _ => (),
    }
    Ok(())
}
