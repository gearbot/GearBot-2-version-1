use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;

use crate::{Error, gearbot_info};
use crate::core::Context;
use crate::gears::basic;

pub async fn handle_event(shard_id: &u64, event: &Event, ctx: Arc<Context<'_>>) -> Result<(), Error> {
    match &event {
        Event::MessageCreate(msg) if !msg.author.bot => {
            info!("Received a message from {}, saying {}", msg.author.name, msg.content);
            if let Some(command) = ctx.command_parser.parse(&msg.content) {
                let args = command.arguments.as_str();
                match command.name {
                    "ping" => basic::ping(&ctx, &msg).await?,
                    "about" => basic::about(&ctx, &msg).await?,
                    "echo" => basic::echo(&ctx, &msg, args).await?,
                    _ => (),
                }

                // TODO: Recognize custom commands.
                ctx.stats.command_used(false).await
            }
        },
        _ => (),
    }
    Ok(())
}