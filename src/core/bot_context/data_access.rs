use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight_model::{
    channel::Message,
    id::{GuildId, MessageId, UserId},
};

use super::BotContext;
use crate::cache::CachedUser;
use crate::core::GuildConfig;
use crate::database::structures::UserMessage;
use crate::error::{DatabaseError, ParseError};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum UserHolder {
    Valid(CachedUser),
    Invalid,
}

const USER_CACHE_DURATION: u32 = 3600;

impl BotContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<CachedUser>, ParseError> {
        if let Some(user) = self.cache.get_user(user_id) {
            return Ok(user);
        }

        //try to find them in redis
        let redis_key = format!("user:{}", user_id);
        let redis_cache = &self.datastore.cache_pool;
        if let Some(user_holder) = redis_cache.get::<UserHolder>(&redis_key).await? {
            return match user_holder {
                UserHolder::Valid(user) => Ok(Arc::new(user)),
                UserHolder::Invalid => Err(ParseError::InvalidUserID(user_id.0)),
            };
        }

        // let's see if we can get em from the api
        let user = self.http.user(user_id).await?;

        match user {
            Some(user) => {
                let user = CachedUser::from_user(&user);
                redis_cache
                    .set(
                        &redis_key,
                        &UserHolder::Valid { 0: user.clone() },
                        Some(USER_CACHE_DURATION),
                    )
                    .await?;
                Ok(Arc::new(user))
            }
            None => {
                redis_cache
                    .set(&redis_key, &UserHolder::Invalid, Some(USER_CACHE_DURATION))
                    .await?;
                Err(ParseError::InvalidUserID(user_id.0))
            }
        }
    }

    pub async fn get_config(&self, guild_id: GuildId) -> Result<Arc<GuildConfig>, DatabaseError> {
        // Clone the option so we can release the lock much faster
        let config = self.configs.read().await.get(&guild_id).cloned();
        match config {
            Some(config) => Ok(config),
            None => {
                let datastore = &self.datastore;
                let config = match datastore.get_guild_config(guild_id.0).await? {
                    Some(c) => c,
                    None => datastore.create_new_guild_config(guild_id.0).await?,
                };

                let config = Arc::new(config);
                self.configs.write().await.insert(guild_id, Arc::clone(&config));
                Ok(config)
            }
        }
    }

    pub async fn set_config(&self, guild_id: GuildId, config: GuildConfig) -> Result<(), DatabaseError> {
        //TODO: validate values? or do we leave that to whoever edited it?
        self.datastore.set_guild_config(guild_id.0, &config).await?;
        self.configs.write().await.insert(guild_id, Arc::new(config));
        Ok(())
    }

    pub async fn fetch_user_message(
        &self,
        message_id: MessageId,
        guild_id: GuildId,
    ) -> Result<Option<UserMessage>, DatabaseError> {
        self.datastore.get_full_message(message_id, guild_id).await
    }

    pub async fn insert_message(&self, message: &Message, guild_id: GuildId) -> Result<(), DatabaseError> {
        // All guilds need to have a config before anything can happen thanks to encryption.
        let _ = self.get_config(guild_id).await?;

        let datastore = &self.datastore;

        datastore.insert_message(&message, guild_id).await?;

        for attachment in &message.attachments {
            datastore.insert_attachment(message.id, attachment).await?;
        }

        Ok(())
    }
}
