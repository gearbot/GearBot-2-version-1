use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;

use crate::core::Context;
use crate::gears::basic;
use crate::parser::parser::Parser;
use crate::utils::errors::Error;

pub async fn handle_event<'a>(event: Event, ctx: Arc<Context>) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) if !msg.author.bot => {
            info!(
                "Received a message from {}, saying {}",
                msg.author.name, msg.content
            );
            Parser::figure_it_out(msg, ctx).await?;



            // if let Some(command) = ctx.command_parser.parse(&msg.content) {
            //     let args = command.arguments;
            //     match command.name {
            //         "about" => basic::about(&ctx, &msg).await?,
            //         "coinflip" => basic::coinflip(&ctx, &msg, &args).await?,
            //         "echo" => basic::echo(&ctx, &msg, &args).await?,
            //         "ping" => basic::ping(&ctx, &msg).await?,
            //         _ => (),
            //     }
            //
            //     // TODO: Recognize custom commands.
            //     ctx.stats.command_used(false).await
            // }
        }
        _ => (),
    }
    Ok(())
}
