use std::sync::Arc;

use rand;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;
use crate::utils;

pub async fn coinflip(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    let thing_todo: String = parser
        .parts
        .into_iter()
        .skip(1)
        .collect::<Vec<String>>()
        .join(" ");

    let thing_todo = if !thing_todo.is_empty() {
        utils::clean(&thing_todo, true, true, true, true)
    } else {
        ctx.http
            .create_message(msg.channel_id)
            .content("You didn't give me anything to flip on!")
            .await?;

        return Ok(());
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
