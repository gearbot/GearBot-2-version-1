use std::sync::Arc;

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;
use crate::utils;

pub async fn echo(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    let ec: Vec<String> = parser.parts.into_iter().skip(1).collect();
    let echo_contents = utils::clean(&ec.join(" "), true, true, true, true);
    ctx.http
        .create_message(msg.channel_id)
        .content(echo_contents)
        .await
        .unwrap();

    Ok(())
}
