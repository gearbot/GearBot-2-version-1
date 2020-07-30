use crate::core::{BotContext, GuildConfig};
use crate::crypto::{self, EncryptionKey};
use crate::utils::Error;

pub async fn get_guild_config(ctx: &BotContext, guild_id: u64) -> Result<Option<GuildConfig>, Error> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT config from guildconfig where id=$1")
        .bind(guild_id as i64)
        .fetch_optional(&ctx.pool)
        .await?;

    let config = if let Some(c_val) = row {
        Some(serde_json::from_value(c_val.0).unwrap())
    } else {
        None
    };

    Ok(config)
}

pub async fn create_new_guild_config(
    ctx: &BotContext,
    guild_id: u64,
    master_ek: &EncryptionKey,
) -> Result<GuildConfig, Error> {
    let new_config = GuildConfig::default();

    let guild_encryption_key = crypto::generate_guild_encryption_key(master_ek, guild_id);

    sqlx::query("INSERT INTO guildconfig (id, config, encryption_key) VALUES ($1, $2, $3)")
        .bind(guild_id as i64)
        .bind(serde_json::to_value(&new_config).unwrap())
        .bind(guild_encryption_key)
        .execute(&ctx.pool)
        .await?;

    Ok(new_config)
}

pub async fn set_guild_config(ctx: &BotContext, guild_id: u64, config: serde_json::Value) -> Result<(), Error> {
    sqlx::query("UPDATE guildconfig set config=$1 WHERE id=$2")
        .bind(&config)
        .bind(guild_id as i64)
        .execute(&ctx.pool)
        .await?;

    Ok(())
}
