use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::utils::{matchers, ParseError};

pub async fn uid(mut ctx: CommandContext) -> CommandResult {
    let user_id = {
        let msg = ctx.parser.get_next().ok_or(ParseError::MissingArgument)?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    ctx.reply_raw(user_id).await?;

    Ok(())
}
