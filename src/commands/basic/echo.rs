use std::sync::Arc;

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;

pub async fn echo(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    // TODO: Sanitize this
    let echo_contents: Vec<String> = parser.parts.into_iter().skip(1).collect();
    let echo_contents = echo_contents.join(" ");
    ctx.http
        .create_message(msg.channel_id)
        .content(echo_contents)
        .await
        .unwrap();

    Ok(())
}
