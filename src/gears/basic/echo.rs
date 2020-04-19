use std::sync::Arc;

use twilight::command_parser::Arguments;
use twilight::model::channel::Message;

use crate::core::Context;
use crate::CommandResult;

pub async fn echo(ctx: &Arc<Context<'_>>, msg: &Message, args: &Arguments<'_>) -> CommandResult {
    let echoed_msg = args.as_str();
    ctx.http
        .create_message(msg.channel_id)
        .content(echoed_msg)
        .await
        .unwrap();

    Ok(())
}
