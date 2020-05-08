use crate::core::{Context, GuildConfig};
use crate::database::guild::{get_guild_config, set_guild_config};
use crate::utils::Error;
use dashmap::mapref::one::Ref;
use serde_json::to_value;
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

    pub async fn set_config(&self, guild_id: GuildId, config: GuildConfig) -> Result<(), Error> {
        //TODO: validate values? or do we leave that to whoever edited it?
        set_guild_config(&self, guild_id.0, to_value(&config)?).await?;
        self.configs.insert(guild_id, config);
        Ok(())
    }
}
