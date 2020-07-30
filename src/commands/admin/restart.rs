use crate::core::CommandContext;
use crate::gearbot_important;
use crate::CommandResult;

pub async fn restart(ctx: CommandContext) -> CommandResult {
    ctx.bot_context
        .http
        .create_message(ctx.message.channel.get_id())
        .content("Shutting down")?
        .await?;

    gearbot_important!("Reboot initiated by {}", ctx.message.author.username);
    ctx.initiate_cold_resume().await?;
    Ok(())
}
