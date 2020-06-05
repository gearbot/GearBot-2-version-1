use rand;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils;

pub async fn coinflip(ctx: CommandContext, msg: Message, parser: Parser) -> CommandResult {
    let thing_todo: String = parser
        .parts
        .into_iter()
        .skip(1)
        .collect::<Vec<String>>()
        .join(" ");

    let thing_todo = if !thing_todo.is_empty() {
        utils::clean(&thing_todo, true, true, true, true)
    } else {
        ctx.send_message("You didn't give me anything to flip on!", msg.channel_id)
            .await?;
        return Ok(());
    };

    let message_text = if rand::random() {
        format!("Yes, you should absolutely {}", thing_todo)
    } else {
        format!("No, you should probably not {}", thing_todo)
    };

    ctx.send_message(message_text, msg.channel_id).await?;

    Ok(())
}
