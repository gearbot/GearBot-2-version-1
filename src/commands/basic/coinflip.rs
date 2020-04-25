use std::sync::Arc;

use rand;
use twilight::command_parser::Arguments;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::utils::errors::Error;

pub async fn coinflip(ctx: &Arc<Context>, msg: &Message, args: &Arguments<'_>) -> CommandResult {
    // TODO: This needs sanatized with the clean function.
    let thing_todo = args.as_str();

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
