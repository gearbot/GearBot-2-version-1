pub mod configs;
mod redis;
pub use redis::api_structs;
pub use redis::Redis;

pub mod structures;

use structures::{StoredUserMessage, UserMessage};

use twilight_model::channel::{Attachment, Message};
use twilight_model::id::{ChannelId, GuildId, MessageId, UserId};

use log::info;

use crate::error::{DatabaseError, StartupError};
use crate::BotConfig;
use crate::{
    crypto::{self, EncryptionKey},
    gearbot_error, gearbot_info,
};

/// An abstraction over the persistent backing storage of the Bot (SQL) and the Redis cache that lives inbetween.
///
/// All database access should go through here.
pub struct DataStorage {
    persistent_pool: sqlx::PgPool,
    pub cache_pool: Redis,
    primary_encryption_key: EncryptionKey<'static>,
}

impl DataStorage {
    /// Initalizes the storage subsystem of GearBot.
    ///
    /// Creates a connection pool with the SQL server and the Redis
    /// in-memory cache.
    ///
    /// While connecting to the SQL server, any required migrations will be ran
    /// before returning.
    pub async fn initalize(config: &BotConfig) -> Result<Self, StartupError> {
        let postgres_pool = match sqlx::Pool::connect(&config.database.postgres).await {
            Ok(pool) => pool,
            Err(e) => {
                gearbot_error!("Failed to connect to the Postgres server: {}", e);
                return Err(StartupError::Sqlx(e));
            }
        };

        info!("Connected to Postgres!");

        info!("Handling database migrations...");
        if let Err(e) = sqlx::migrate!("./migrations").run(&postgres_pool).await {
            gearbot_error!("Failed to run SQL migrations: {}", e);
            return Err(StartupError::Sqlx(e.into()));
        }

        info!("Finished migrations!");

        let redis_pool = match Redis::new(&config.database.redis).await {
            Ok(pool) => pool,
            Err(e) => {
                gearbot_error!("Failed to connect to the Redis cache: {}", e);
                return Err(StartupError::DarkRedis(e));
            }
        };

        info!("Connected to Redis");

        gearbot_info!("Database connections established");

        Ok(Self {
            persistent_pool: postgres_pool,
            cache_pool: redis_pool,
            primary_encryption_key: EncryptionKey::construct_owned(&config.main_encryption_key),
        })
    }

    pub async fn insert_message(&self, message: &Message, guild_id: GuildId) -> Result<(), DatabaseError> {
        let start = std::time::Instant::now();

        let ciphertext = {
            let plaintext = message.content.as_bytes();

            let guild_key = self.get_guild_encryption_key(guild_id).await?;
            crypto::encrypt_bytes(plaintext, &guild_key, message.id.0)
        };

        info!("It took {}us to encrypt the user message!", start.elapsed().as_micros());

        sqlx::query(
            "INSERT INTO message (id, encrypted_content, author_id, channel_id, guild_id, kind, pinned)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(message.id.0 as i64)
        .bind(ciphertext)
        .bind(message.author.id.0 as i64)
        .bind(message.channel_id.0 as i64)
        .bind(message.guild_id.unwrap().0 as i64)
        .bind(message.kind as i16)
        .bind(message.pinned)
        .execute(&self.persistent_pool)
        .await?;

        Ok(())
    }

    pub async fn insert_attachment(&self, message_id: MessageId, attachment: &Attachment) -> Result<(), DatabaseError> {
        sqlx::query(
            "INSERT INTO attachment (id, name, image, message_id)
            VALUES ($1, $2, $3, $4)",
        )
        .bind(attachment.id.0 as i64)
        .bind(&attachment.filename)
        .bind(attachment.width.is_some())
        .bind(message_id.0 as i64)
        .execute(&self.persistent_pool)
        .await?;

        Ok(())
    }

    pub async fn get_full_message(
        &self,
        message_id: MessageId,
        guild_id: GuildId,
    ) -> Result<Option<UserMessage>, DatabaseError> {
        let stored_message: Option<StoredUserMessage> = sqlx::query_as("SELECT * from message where id=$1")
            .bind(message_id.0 as i64)
            .fetch_optional(&self.persistent_pool)
            .await?;

        let user_msg = match stored_message {
            Some(sm) => {
                let start = std::time::Instant::now();

                let guild_key = self.get_guild_encryption_key(guild_id).await?;
                let decrypted_content = crypto::decrypt_bytes(&sm.encrypted_content, &guild_key, message_id.0);

                info!("It took {}us to decrypt a user message!", start.elapsed().as_micros());

                Some(UserMessage {
                    content: String::from_utf8(decrypted_content).unwrap(),
                    author: UserId(sm.author_id as u64),
                    channel: ChannelId(sm.channel_id as u64),
                    guild: GuildId(sm.guild_id as u64),
                    kind: sm.kind(),
                    pinned: sm.pinned,
                })
            }
            None => None,
        };

        Ok(user_msg)
    }

    async fn get_guild_encryption_key(&self, guild_id: GuildId) -> Result<EncryptionKey<'_>, DatabaseError> {
        let ek_bytes: (Vec<u8>,) = sqlx::query_as("SELECT encryption_key from guildconfig where id=$1")
            .bind(guild_id.0 as i64)
            .fetch_one(&self.persistent_pool)
            .await?;

        let guild_key = {
            let main_ek = &self.primary_encryption_key;
            let decrypted_gk_bytes = crypto::decrypt_bytes(&ek_bytes.0, main_ek, guild_id.0);
            EncryptionKey::construct_owned(&decrypted_gk_bytes)
        };

        Ok(guild_key)
    }
}
