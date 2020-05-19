use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::GuildContext;
use crate::parser::Parser;
use crate::utils;

pub async fn echo(ctx: GuildContext, msg: Message, parser: Parser) -> CommandResult {
    let ec: Vec<String> = parser.parts.into_iter().skip(1).collect();
    let echo_contents = utils::clean(&ec.join(" "), true, true, true, true);

    ctx.send_message(echo_contents, msg.channel_id).await?;

    Ok(())
}
