use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::utils::{matchers, ParseError};

pub async fn uid(mut ctx: CommandContext) -> CommandResult {
    let user_id = {
        let msg = ctx.parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    //TODO: make this actually scan and find all
    ctx.reply_raw(user_id.to_string()).await?;

    Ok(())
}
