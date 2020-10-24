use crate::core::{CommandContext, GuildConfig};
use crate::utils::DatabaseError;
use crate::CommandResult;

pub async fn get_config(ctx: CommandContext) -> CommandResult {
    let stringified_config = serde_json::to_string(&ctx.get_config()?).map_err(|e| DatabaseError::Deserializing(e))?;

    ctx.reply_raw(stringified_config).await?;

    Ok(())
}

pub async fn get_config_pretty(ctx: CommandContext) -> CommandResult {
    let stringified_config =
        serde_json::to_string_pretty(&ctx.get_config()?).map_err(|e| DatabaseError::Deserializing(e))?;

    ctx.reply_raw(format!("```json\n{}```", stringified_config)).await?;

    Ok(())
}

pub async fn set_config(mut ctx: CommandContext) -> CommandResult {
    let config: GuildConfig =
        serde_json::from_str(&ctx.parser.get_remaining()).map_err(|e| DatabaseError::Deserializing(e))?;
    ctx.set_config(config).await?;
    ctx.reply_raw("Config updated").await?;
    Ok(())
}

pub async fn reset_config(ctx: CommandContext) -> CommandResult {
    ctx.set_config(GuildConfig::default()).await?;
    ctx.reply_raw("Config reset to default").await?;
    Ok(())
}
