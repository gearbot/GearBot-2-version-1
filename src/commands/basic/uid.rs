use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils::{matchers, ParseError};

pub async fn uid(ctx: CommandContext, mut parser: Parser) -> CommandResult {
    let user_id = {
        let msg = parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    //TODO: make this actually scan and find all
    ctx.reply_raw(user_id.to_string()).await?;

    Ok(())
}
