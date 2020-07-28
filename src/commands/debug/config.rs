use crate::core::{CommandContext, GuildConfig};
use crate::parser::Parser;
use crate::CommandResult;

pub async fn get_config(ctx: CommandContext, _: Parser) -> CommandResult {
    let stringified_config = serde_json::to_string(ctx.get_config()?)?;

    ctx.reply_raw(stringified_config).await?;

    Ok(())
}

pub async fn get_config_pretty(ctx: CommandContext, _: Parser) -> CommandResult {
    let stringified_config = serde_json::to_string_pretty(ctx.get_config()?)?;

    ctx.reply_raw(format!("```json\n{}```", stringified_config)).await?;

    Ok(())
}

pub async fn set_config(ctx: CommandContext, mut parser: Parser) -> CommandResult {
    let config: GuildConfig = serde_json::from_str(&*parser.get_remaining())?;
    ctx.set_config(config).await?;
    Ok(())
}
