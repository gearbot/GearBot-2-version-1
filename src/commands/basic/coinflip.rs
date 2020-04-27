use std::sync::Arc;

use rand;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;

pub async fn coinflip(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    // TODO: This needs sanatized with the clean function.
    let thing_todo = match parser.parts.into_iter().nth(1) {
        Some(thing) => thing,
        None => {
            ctx.http
                .create_message(msg.channel_id)
                .content("You didn't give me anything to flip on!")
                .await?;

            return Ok(());
        }
    };

    let message_text = if rand::random() {
        format!("Yes, you should absolutely {}", thing_todo)
    } else {
        format!("No, you should probably not {}", thing_todo)
    };

    ctx.http
        .create_message(msg.channel_id)
        .content(message_text)
        .await?;

    Ok(())
}
