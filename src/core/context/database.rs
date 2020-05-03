use crate::core::{Context, GuildConfig};
use crate::utils::Error;
use dashmap::mapref::one::Ref;
use log::{debug, info};
use postgres_types::Type;
use twilight::model::id::GuildId;

impl Context {
    pub async fn get_config(
        &self,
        guild_id: GuildId,
    ) -> Result<Ref<'_, GuildId, GuildConfig>, Error> {
        match self.configs.get(&guild_id) {
            Some(config) => Ok(config),
            None => {
                let client = self.pool.get().await?;
                let statement = client
                    .prepare_typed("SELECT config from guildconfig where id=$1", &[Type::INT8])
                    .await?;

                let rows = client.query(&statement, &[&(guild_id.0 as i64)]).await?;

                let config: GuildConfig = if rows.is_empty() {
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
                                &(guild_id.0 as i64),
                                &serde_json::to_value(&GuildConfig::default()).unwrap(),
                            ],
                        )
                        .await?;

                    config
                } else {
                    serde_json::from_value(rows[0].get(0))?
                };

                self.configs.insert(guild_id, config);
                Ok(self.configs.get(&guild_id).unwrap())
            }
        }
    }
}
