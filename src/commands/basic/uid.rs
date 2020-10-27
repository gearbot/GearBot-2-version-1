use crate::core::CommandContext;
use crate::error::{CommandResult, ParseError};
use crate::utils::matchers;

pub async fn uid(mut ctx: CommandContext) -> CommandResult {
    let user_id = {
        let msg = ctx.parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    ctx.reply_raw(user_id).await?;

    Ok(())
}
