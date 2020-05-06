use crate::core::{Context, GuildConfig};
use crate::utils::Error;
use deadpool_postgres::Pool;
use log::info;
use postgres_types::Type;
use rand::{thread_rng, RngCore};

pub async fn get_guild_config(ctx: &Context, guild_id: u64) -> Result<GuildConfig, Error> {
    let client = ctx.pool.get().await?;
    let statement = client
        .prepare_typed("SELECT config from guildconfig where id=$1", &[Type::INT8])
        .await?;

    let rows = client.query(&statement, &[&(guild_id as i64)]).await?;

    if rows.is_empty() {
        let config = GuildConfig::default();
        info!("No config found for {}, inserting blank one", guild_id);
        let statement = client
            .prepare_typed(
                "INSERT INTO guildconfig (id, config, encryption_key) VALUES ($1, $2, $3)",
                &[Type::INT8, Type::JSON, Type::BYTEA],
            )
            .await?;
        client
            .execute(
                &statement,
                &[
                    &(guild_id as i64),
                    &serde_json::to_value(&GuildConfig::default()).unwrap(),
                    &ctx.generate_guild_key(guild_id),
                ],
            )
            .await?;

        Ok(config)
    } else {
        Ok(serde_json::from_value(rows[0].get(0))?)
    }
}
