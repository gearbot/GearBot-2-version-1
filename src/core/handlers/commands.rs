use std::sync::Arc;

use log::{debug, info};
use twilight::gateway::cluster::Event;

use crate::core::Context;
use crate::parser::Parser;
use crate::utils::Error;
use std::borrow::Borrow;

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
            let mut prefix = String::from("!");
            if msg.guild_id.is_some() {
                let config = ctx.get_config(msg.guild_id.unwrap().0 as i64).await?;
                prefix = config.value().prefix.clone()
            }
            debug!(
                "prefix: {}, starts: {}",
                prefix,
                msg.content.starts_with(&prefix)
            );
            if msg.content.starts_with(&prefix) {
                Parser::figure_it_out(&prefix, msg, ctx).await?;
            }
        }
        _ => (),
    }
    Ok(())
}
