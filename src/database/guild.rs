use crate::core::GuildConfig;
use crate::utils::Error;
use deadpool_postgres::Pool;
use log::info;
use postgres_types::Type;

pub async fn get_guild_config(pool: &Pool, guild_id: u64) -> Result<GuildConfig, Error> {
    let client = pool.get().await?;
    let statement = client
        .prepare_typed("SELECT config from guildconfig where id=$1", &[Type::INT8])
        .await?;

    let rows = client.query(&statement, &[&(guild_id as i64)]).await?;

    if rows.is_empty() {
        let config = GuildConfig::default();
        info!("No config found for {}, inserting blank one", guild_id);
        let statement = client
            .prepare_typed(
                "INSERT INTO guildconfig (id, config) VALUES ($1, $2)",
                &[Type::INT8, Type::JSON],
            )
            .await?;
        client
            .execute(
                &statement,
                &[
                    &(guild_id as i64),
                    &serde_json::to_value(&GuildConfig::default()).unwrap(),
                ],
            )
            .await?;

        Ok(config)
    } else {
        Ok(serde_json::from_value(rows[0].get(0))?)
    }
}
