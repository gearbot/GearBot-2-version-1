use crate::core::{Context, GuildConfig};
use crate::database::guild::get_guild_config;
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
                let config = get_guild_config(&self, guild_id.0).await?;
                self.configs.insert(guild_id, config);
                Ok(self.configs.get(&guild_id).unwrap())
            }
        }
    }
}
