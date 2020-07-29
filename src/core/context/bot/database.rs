use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, NewAead},
    Aes256Gcm,
};
use dashmap::ElementGuard;
use log::info;
use rand::{thread_rng, RngCore};
use serde_json::to_value;
use twilight::model::channel::message::MessageType;
use twilight::model::channel::Message;
use twilight::model::id::{ChannelId, GuildId, MessageId, UserId};

use crate::core::{BotContext, GuildConfig};
use crate::database::cache::{get_channel_for_message, get_full_message};
use crate::database::guild::{get_guild_config, set_guild_config};
use crate::utils::Error;
use crate::{database, EncryptionKey};

#[derive(Debug)]
pub struct UserMessage {
    pub content: String,
    pub author: UserId,
    pub channel: ChannelId,
    pub guild: GuildId,
    pub msg_type: MessageType,
    pub pinned: bool,
}

impl BotContext {
    pub async fn get_config(&self, guild_id: GuildId) -> Result<ElementGuard<GuildId, GuildConfig>, Error> {
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

    pub async fn fetch_user_message(&self, id: MessageId) -> Result<Option<UserMessage>, Error> {
        if let Some((encrypted, author_id, channel_id, guild_id, msg_type, pinned)) =
            get_full_message(&self.pool, id.0).await?
        {
            let guild_id = GuildId(guild_id);
            let start = std::time::Instant::now();
            let decyrpted = {
                let guild_key = self.get_guild_encryption_key(guild_id).await?;

                decrypt_bytes(encrypted.as_slice(), &guild_key, id.0)
            };

            let fin = std::time::Instant::now();

            info!("It took {}us to decrypt a user message!", (fin - start).as_micros());

            Ok(Some(UserMessage {
                content: String::from_utf8_lossy(&decyrpted).to_string(),
                author: UserId(author_id),
                channel: ChannelId(channel_id),
                guild: guild_id,
                msg_type,
                pinned,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn insert_message(&self, msg: &Message, guild_id: GuildId) -> Result<(), Error> {
        // All guilds need to have a config before anything can happen thanks to encryption.
        let _ = self.get_config(guild_id).await?;

        let msg_id = msg.id.0 as i64;

        let start = std::time::Instant::now();

        let ciphertext = {
            let plaintext = msg.content.as_bytes();
            let guild_key = self.get_guild_encryption_key(guild_id).await?;

            encrypt_bytes(plaintext, &guild_key, msg_id as u64)
        };

        let finish_crypto = std::time::Instant::now();

        info!(
            "It took {}us to encrypt the user message!",
            (finish_crypto - start).as_micros()
        );

        database::cache::insert_message(&self.pool, ciphertext, &msg).await?;
        for attachment in &msg.attachments {
            database::cache::insert_attachment(&self.pool, msg.id.0, attachment).await?;
        }

        Ok(())
    }

    async fn get_guild_encryption_key(&self, guild_id: GuildId) -> Result<EncryptionKey, Error> {
        let ek_bytes: (Vec<u8>,) = sqlx::query_as("SELECT encryption_key from guildconfig where id=$1")
            .bind(guild_id.0 as i64)
            .fetch_one(&self.pool)
            .await?;

        let guild_key = {
            let master_key = self.__get_master_key().unwrap();

            let decrypted_gk_bytes = decrypt_bytes(&ek_bytes.0, master_key, guild_id.0);
            EncryptionKey::clone_from_slice(&decrypted_gk_bytes)
        };

        Ok(guild_key)
    }

    pub fn generate_guild_key(&self, guild_id: u64) -> Vec<u8> {
        //TODO: check how crypto safe this is
        let mut csprng = thread_rng();
        // Each guild has its own encryption key. This allows us, in the event of a compromise of the master key,
        // to simply re-encrypt the guild keys instead of millions of messages.
        let mut guild_encryption_key = [0u8; 32];
        csprng.fill_bytes(&mut guild_encryption_key);

        let master_key = self.__get_master_key().unwrap();
        encrypt_bytes(&guild_encryption_key, master_key, guild_id)
    }

    pub async fn get_channel_for_message(&self, message_id: u64) -> Result<Option<u64>, Error> {
        get_channel_for_message(&self.pool, message_id).await
    }
}

fn encrypt_bytes(plaintext: &[u8], key: &EncryptionKey, id: u64) -> Vec<u8> {
    let aead = Aes256Gcm::new(key);

    // Since nonce's only never need to be reused, and Discor's snowflakes for messages
    // are unique, we can use the messasge id to construct the nonce with its 64 bits, and then
    // pad the rest with zeros.
    let mut nonce_bytes = [0u8; 12];
    let msg_id_bytes = id.to_le_bytes();
    nonce_bytes[..8].copy_from_slice(&msg_id_bytes);
    nonce_bytes[8..].copy_from_slice(&[0u8; 4]);

    let nonce = GenericArray::from_slice(&nonce_bytes);

    aead.encrypt(&nonce, plaintext).expect("Failed to encrypt an object!")
}

fn decrypt_bytes(ciphertext: &[u8], key: &EncryptionKey, id: u64) -> Vec<u8> {
    let aead = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    let msg_id_bytes = id.to_le_bytes();
    nonce_bytes[..8].copy_from_slice(&msg_id_bytes);
    nonce_bytes[8..].copy_from_slice(&[0u8; 4]);

    let nonce = GenericArray::from_slice(&nonce_bytes);

    aead.decrypt(&nonce, ciphertext).expect("Failed to decrypt an object!")
}
