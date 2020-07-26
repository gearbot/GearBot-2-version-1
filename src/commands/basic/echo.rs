use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;

pub async fn echo(_: CommandContext, _: Parser) -> CommandResult {
    // let ec: Vec<String> = parser.parts.into_iter().skip(1).collect();
    // let echo_contents = utils::clean(&ec.join(" "), true, true, true, true);

    // ctx.reply(echo_contents).await?;

    Ok(())
}
