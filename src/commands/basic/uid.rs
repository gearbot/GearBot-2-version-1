use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils::{matchers, ParseError};

pub async fn uid(ctx: CommandContext, mut parser: Parser) -> CommandResult {
    let user_id = {
        let msg = parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    ctx.reply(user_id.to_string()).await?;

    Ok(())
}
