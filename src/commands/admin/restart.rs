use crate::core::CommandContext;
use crate::gearbot_important;
use crate::parser::Parser;
use crate::CommandResult;

pub async fn restart(ctx: CommandContext, _: Parser) -> CommandResult {
    if ctx.message.author.id.0 == 106354106196570112 {
        ctx.reply("Shutting down").await?;
        gearbot_important!("Reboot initiated by {}", ctx.message.author.username);
        ctx.initiate_cold_resume().await?;
    }
    Ok(())
}
