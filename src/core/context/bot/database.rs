use twilight_model::channel::Message;
use twilight_model::id::{GuildId, MessageId};

use crate::core::{BotContext, GuildConfig};
use crate::database::{self, configs as dbconfig, structures::UserMessage};

use crate::crypto::{self, EncryptionKey};
use crate::error::DatabaseError;
use std::sync::Arc;

impl BotContext {
    pub async fn get_config(&self, guild_id: GuildId) -> Result<Arc<GuildConfig>, DatabaseError> {
        //clone the option so we can release the lock much faster
        let config = self.configs.read().await.get(&guild_id).cloned();
        match config {
            Some(config) => Ok(config),
            None => {
                let master_ek = self.__get_main_encryption_key();

                let config = match dbconfig::get_guild_config(&self, guild_id.0).await? {
                    Some(c) => c,
                    None => dbconfig::create_new_guild_config(&self, guild_id.0, master_ek).await?,
                };
                let arc = Arc::new(config);
                self.configs.write().await.insert(guild_id, arc.clone());
                Ok(arc)
            }
        }
    }

    pub async fn set_config(&self, guild_id: GuildId, config: GuildConfig) -> Result<(), DatabaseError> {
        //TODO: validate values? or do we leave that to whoever edited it?
        dbconfig::set_guild_config(&self, guild_id.0, &config).await?;
        self.configs.write().await.insert(guild_id, Arc::new(config));
        Ok(())
    }

    pub async fn fetch_user_message(
        &self,
        message_id: MessageId,
        guild_id: GuildId,
    ) -> Result<Option<UserMessage>, DatabaseError> {
        let guild_key = self.get_guild_encryption_key(guild_id).await?;
        database::get_full_message(&self.backing_database, message_id, &guild_key).await
    }

    pub async fn insert_message(&self, msg: &Message, guild_id: GuildId) -> Result<(), DatabaseError> {
        // All guilds need to have a config before anything can happen thanks to encryption.
        let _ = self.get_config(guild_id).await?;
        let guild_key = self.get_guild_encryption_key(guild_id).await?;

        database::insert_message(&self.backing_database, &msg, &guild_key).await?;
        for attachment in &msg.attachments {
            database::insert_attachment(&self.backing_database, msg.id, attachment).await?;
        }

        Ok(())
    }

    async fn get_guild_encryption_key(&self, guild_id: GuildId) -> Result<EncryptionKey, DatabaseError> {
        let ek_bytes: (Vec<u8>,) = sqlx::query_as("SELECT encryption_key from guildconfig where id=$1")
            .bind(guild_id.0 as i64)
            .fetch_one(&self.backing_database)
            .await?;

        let guild_key = {
            let master_key = self.__get_main_encryption_key();

            let decrypted_gk_bytes = crypto::decrypt_bytes(&ek_bytes.0, master_key, guild_id.0);
            EncryptionKey::clone_from_slice(&decrypted_gk_bytes)
        };

        Ok(guild_key)
    }
}
