use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils;

pub async fn echo(ctx: CommandContext, parser: Parser) -> CommandResult {
    let ec: Vec<String> = parser.parts.into_iter().skip(1).collect();
    let echo_contents = utils::clean(&ec.join(" "), true, true, true, true);

    ctx.reply(echo_contents).await?;

    Ok(())
}
