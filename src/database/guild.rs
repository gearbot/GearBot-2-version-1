use log::info;

use crate::core::{BotContext, GuildConfig};
use crate::utils::Error;

pub async fn get_guild_config(ctx: &BotContext, guild_id: u64) -> Result<GuildConfig, Error> {
    let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT config from guildconfig where id=$1")
        .bind(guild_id as i64)
        .fetch_optional(&ctx.pool)
        .await?;

    let config = match row {
        Some(cv) => serde_json::from_value(cv.0).unwrap(),
        None => {
            let config = GuildConfig::default();
            info!("No config found for {}, inserting blank one", guild_id);

            sqlx::query("INSERT INTO guildconfig (id, config, encryption_key) VALUES ($1, $2, $3)")
                .bind(guild_id as i64)
                .bind(&serde_json::to_value(&config).unwrap())
                .bind(&ctx.generate_guild_key(guild_id))
                .execute(&ctx.pool)
                .await?;

            config
        }
    };

    Ok(config)
}

pub async fn set_guild_config(ctx: &BotContext, guild_id: u64, config: serde_json::Value) -> Result<(), Error> {
    sqlx::query("UPDATE guildconfig set config=$1 WHERE id=$2")
        .bind(&config)
        .bind(guild_id as i64)
        .execute(&ctx.pool)
        .await?;

    Ok(())
}
