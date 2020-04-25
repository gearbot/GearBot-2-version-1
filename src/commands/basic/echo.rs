use std::sync::Arc;

use twilight::command_parser::Arguments;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::parser::Parser;
use crate::utils::errors::Error;

pub async fn echo(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    ctx.http
        .create_message(msg.channel_id)
        .content("same")
        .await
        .unwrap();

    Ok(())
}
