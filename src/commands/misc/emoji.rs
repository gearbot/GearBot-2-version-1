use crate::core::{CommandContext, Reactor};
use crate::CommandResult;

pub async fn emoji_list(ctx: CommandContext) -> CommandResult {
    let reactor = Reactor::new_emoji_list();
    let message = ctx.reply_raw("Test reactor").await?;
    reactor.save(&ctx.bot_context, message.id).await?;
    Ok(())
}

pub async fn emoji_info(ctx: CommandContext) -> CommandResult {
    Ok(())
}
