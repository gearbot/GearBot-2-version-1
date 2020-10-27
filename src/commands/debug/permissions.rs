use crate::core::CommandContext;
use crate::error::CommandResult;

pub async fn get_perms(mut ctx: CommandContext) -> CommandResult {
    let member = ctx.parser.get_member_or(ctx.message.get_author_as_member()?)?;
    let guild = ctx.get_guild()?;
    let config = ctx.get_config()?;
    ctx.reply_raw(format!(
        "```{:?}```",
        ctx.bot_context.get_permissions_for(&guild, &member, &config)
    ))
    .await?;
    Ok(())
}
