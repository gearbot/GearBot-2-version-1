use crate::core::CommandContext;
use crate::gearbot_important;
use crate::parser::Parser;
use crate::CommandResult;

pub async fn restart(ctx: CommandContext, _: Parser) -> CommandResult {
    if ctx.message.author.id.0 == 106354106196570112 {
        ctx.bot_context
            .http
            .create_message(ctx.message.channel.get_id())
            .content("Shutting down")?
            .await?;
        gearbot_important!("Reboot initiated by {}", ctx.message.author.username);
        ctx.initiate_cold_resume().await?;
    }
    Ok(())
}
