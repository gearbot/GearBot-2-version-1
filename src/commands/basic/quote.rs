use twilight_model::id::MessageId;

use crate::core::CommandContext;
use crate::error::{CommandResult, ParseError};
use crate::translation::{FluArgs, GearBotString};
use crate::utils;

pub async fn quote(mut ctx: CommandContext) -> CommandResult {
    let msg_id = ctx
        .parser
        .get_next()?
        .parse::<u64>()
        .map_err(|_| ParseError::MissingArgument)?;

    let guild_id = ctx.get_guild().id;

    match ctx.bot_context.fetch_user_message(MessageId(msg_id), guild_id).await? {
        Some(msg) => {
            let message = utils::clean(&msg.content, true, true, false, false);
            ctx.reply_raw(message).await?;
        }
        None => {
            let args = FluArgs::with_capacity(0).generate();
            ctx.reply(GearBotString::QuoteNotFound, args).await?;
        }
    }

    Ok(())
}
