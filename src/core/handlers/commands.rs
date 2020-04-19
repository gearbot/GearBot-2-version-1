use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;

use crate::core::Context;
use crate::gears::basic;
use crate::Error;

// TODO: How to use this to make sure we have registered all of them.
// Maybe a macro could check the match statements arms?
pub const COMMAND_LIST: [&str; 4] = ["about", "ping", "echo", "coinflip"];

pub async fn handle_event(event: &Event, ctx: Arc<Context<'_>>) -> Result<(), Error> {
    match &event {
        Event::MessageCreate(msg) if !msg.author.bot => {
            info!(
                "Received a message from {}, saying {}",
                msg.author.name, msg.content
            );
            if let Some(command) = ctx.command_parser.parse(&msg.content) {
                let args = command.arguments;
                match command.name {
                    "about" => basic::about(&ctx, &msg).await?,
                    "coinflip" => basic::coinflip(&ctx, &msg, &args).await?,
                    "echo" => basic::echo(&ctx, &msg, &args).await?,
                    "ping" => basic::ping(&ctx, &msg).await?,
                    _ => (),
                }

                // TODO: Recognize custom commands.
                ctx.stats.command_used(false).await
            }
        }
        _ => (),
    }
    Ok(())
}
