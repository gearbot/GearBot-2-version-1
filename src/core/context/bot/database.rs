use dashmap::ElementGuard;
use serde_json::to_value;
use twilight::model::channel::Message;
use twilight::model::id::{GuildId, MessageId};

use crate::core::{BotContext, GuildConfig};
use crate::database::{self, configs as dbconfig, structures::UserMessage};

use crate::crypto::{self, EncryptionKey};
use crate::utils::Error;

impl BotContext {
    pub async fn get_config(&self, guild_id: GuildId) -> Result<ElementGuard<GuildId, GuildConfig>, Error> {
        match self.configs.get(&guild_id) {
            Some(config) => Ok(config),
            None => {
                let master_ek = self.__get_main_encryption_key();

                let config = match dbconfig::get_guild_config(&self, guild_id.0).await? {
                    Some(c) => c,
                    None => dbconfig::create_new_guild_config(&self, guild_id.0, master_ek).await?,
                };

                self.configs.insert(guild_id, config);
                Ok(self.configs.get(&guild_id).unwrap())
            }
        }
    }

    pub async fn set_config(&self, guild_id: GuildId, config: GuildConfig) -> Result<(), Error> {
        //TODO: validate values? or do we leave that to whoever edited it?
        dbconfig::set_guild_config(&self, guild_id.0, to_value(&config)?).await?;
        self.configs.insert(guild_id, config);
        Ok(())
    }

    pub async fn fetch_user_message(
        &self,
        message_id: MessageId,
        guild_id: GuildId,
    ) -> Result<Option<UserMessage>, Error> {
        let guild_key = self.get_guild_encryption_key(guild_id).await?;
        database::get_full_message(&self.pool, message_id, &guild_key).await
    }

    pub async fn insert_message(&self, msg: &Message, guild_id: GuildId) -> Result<(), Error> {
        // All guilds need to have a config before anything can happen thanks to encryption.
        let _ = self.get_config(guild_id).await?;
        let guild_key = self.get_guild_encryption_key(guild_id).await?;

        database::insert_message(&self.pool, &msg, &guild_key).await?;
        for attachment in &msg.attachments {
            database::insert_attachment(&self.pool, msg.id, attachment).await?;
        }

        Ok(())
    }

    async fn get_guild_encryption_key(&self, guild_id: GuildId) -> Result<EncryptionKey, Error> {
        let ek_bytes: (Vec<u8>,) = sqlx::query_as("SELECT encryption_key from guildconfig where id=$1")
            .bind(guild_id.0 as i64)
            .fetch_one(&self.pool)
            .await?;

        let guild_key = {
            let master_key = self.__get_main_encryption_key();

            let decrypted_gk_bytes = crypto::decrypt_bytes(&ek_bytes.0, master_key, guild_id.0);
            EncryptionKey::clone_from_slice(&decrypted_gk_bytes)
        };

        Ok(guild_key)
    }
}
