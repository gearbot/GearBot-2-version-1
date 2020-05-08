use crate::core::Context;
use crate::utils::ParseError::MemberNotFoundById;
use crate::utils::{Error, FetchError, ParseError};
use crate::{database, EncryptionKey};
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, NewAead},
    Aes256Gcm,
};
use log::{debug, info, trace};
use postgres_types::Type;
use rand::{thread_rng, RngCore};
use std::sync::Arc;
use tokio::sync::oneshot;
use twilight::http::error::Error::Response;
use twilight::http::error::ResponseError::{Client, Server};
use twilight::http::error::{Error as HttpError, ResponseError};
use twilight::model::channel::message::MessageType;
use twilight::model::channel::Message;
use twilight::model::gateway::payload::{MemberChunk, RequestGuildMembers};
use twilight::model::gateway::presence::Presence;
use twilight::model::guild::Member;
use twilight::model::id::{ChannelId, GuildId, MessageId, UserId};
use twilight::model::user::User;
use uuid::Uuid;

#[derive(Debug)]
pub struct UserMessage {
    pub content: String,
    pub author: UserId,
    pub channel: ChannelId,
    pub guild: GuildId,
    pub msg_type: MessageType,
    pub pinned: bool,
}

impl Context {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<User>, Error> {
        match self.cache.user(user_id).await? {
            Some(user) => Ok(user),
            None => {
                // let's see if we can get em from the api
                let result = self.http.user(user_id.0).await;
                //TODO: cache in redis

                match result {
                    Ok(u) => {
                        let user = u.unwrap(); // there isn't a codepath that can even give none for this atm
                        Ok(Arc::new(user))
                    }
                    Err(error) => {
                        //2 options here:
                        //1) drill down 3 layers and get a headache trying to deal with moving and re-assembling errors to figure out the status code
                        //2) just get the string and find the code in there
                        if format!("{:?}", error).contains("status: 404") {
                            Err(Error::ParseError(ParseError::InvalidUserID(user_id.0)))
                        } else {
                            Err(Error::TwilightHttp(error))
                        }
                    }
                }
            }
        }
    }

    pub async fn fetch_user_message(&self, id: MessageId) -> Result<UserMessage, Error> {
        let client = self.pool.get().await?;

        let statement = client
            .prepare_typed("SELECT * from message where id=$1", &[Type::INT8])
            .await?;

        let fetch_id = id.0 as i64;

        let rows = client.query(&statement, &[&fetch_id]).await?;

        if let Some(stored_msg) = rows.get(0) {
            let encrypted_message: &[u8] = stored_msg.get(1);
            let author: i64 = stored_msg.get(2);
            let channel: i64 = stored_msg.get(3);
            let guild_id = {
                let raw: i64 = stored_msg.get(4);
                GuildId(raw as u64)
            };

            let raw_msg_type: i16 = stored_msg.get(5);
            let pinned = stored_msg.get(6);

            // TODO: This should exist in twilight via a TryFrom
            let msg_type = match raw_msg_type as u8 {
                0 => MessageType::Regular,
                1 => MessageType::RecipientAdd,
                2 => MessageType::RecipientRemove,
                3 => MessageType::Call,
                4 => MessageType::ChannelNameChange,
                5 => MessageType::ChannelIconChange,
                6 => MessageType::ChannelMessagePinned,
                7 => MessageType::GuildMemberJoin,
                8 => MessageType::UserPremiumSub,
                9 => MessageType::UserPremiumSubTier1,
                10 => MessageType::UserPremiumSubTier2,
                11 => MessageType::UserPremiumSubTier3,
                12 => MessageType::ChannelFollowAdd,
                14 => MessageType::GuildDiscoveryDisqualified,
                15 => MessageType::GuildDiscoveryRequalified,
                _ => unimplemented!(),
            };

            let msg_id = fetch_id as u64;

            let start = std::time::Instant::now();

            let plaintext = {
                let guild_key = self.get_guild_encryption_key(guild_id).await?;

                decrypt_bytes(encrypted_message, &guild_key, msg_id)
            };

            let finish = std::time::Instant::now();

            trace!(
                "It took {}ms to decrypt the message!",
                (finish - start).as_millis()
            );

            let plaintext_string = String::from_utf8_lossy(&plaintext).to_string();

            let assembled_message = UserMessage {
                content: plaintext_string,
                author: (author as u64).into(),
                channel: (channel as u64).into(),
                guild: guild_id,
                msg_type,
                pinned,
            };

            Ok(assembled_message)
        } else {
            Err(FetchError::ShouldExist.into())
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

        debug!(
            "It took {}ms to encrypt the user message!",
            (finish_crypto - start).as_millis()
        );

        database::cache::insert_message(&self.pool, ciphertext, &msg).await?;
        for attachment in &msg.attachments {
            database::cache::insert_attachment(&self.pool, msg.id.0, attachment).await?;
        }

        Ok(())
    }

    async fn get_guild_encryption_key(&self, guild_id: GuildId) -> Result<EncryptionKey, Error> {
        let client = self.pool.get().await?;

        let fetch_id = guild_id.0 as i64;

        let statement = client
            .prepare_typed(
                "SELECT encryption_key from guildconfig where id=$1",
                &[Type::INT8],
            )
            .await?;

        let rows = client.query(&statement, &[&fetch_id]).await?;

        if let Some(ek) = rows.get(0) {
            let ek_bytes = ek.get(0);

            let guild_key = {
                let master_key = self.__get_master_key().unwrap();

                let decrypted_gk_bytes = decrypt_bytes(ek_bytes, master_key, fetch_id as u64);
                EncryptionKey::clone_from_slice(&decrypted_gk_bytes)
            };

            Ok(guild_key)
        } else {
            Err(FetchError::ShouldExist.into())
        }
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
}

fn encrypt_bytes(plaintext: &[u8], key: &EncryptionKey, id: u64) -> Vec<u8> {
    let aead = Aes256Gcm::new(*key);

    // Since nonce's only never need to be reused, and Discor's snowflakes for messages
    // are unique, we can use the messasge id to construct the nonce with its 64 bits, and then
    // pad the rest with zeros.
    let mut nonce_bytes = [0u8; 12];
    let msg_id_bytes = id.to_le_bytes();
    nonce_bytes[..8].copy_from_slice(&msg_id_bytes);
    nonce_bytes[8..].copy_from_slice(&[0u8; 4]);

    let nonce = GenericArray::from_slice(&nonce_bytes);

    aead.encrypt(&nonce, plaintext)
        .expect("Failed to encrypt an object!")
}

fn decrypt_bytes(ciphertext: &[u8], key: &EncryptionKey, id: u64) -> Vec<u8> {
    let aead = Aes256Gcm::new(*key);

    let mut nonce_bytes = [0u8; 12];
    let msg_id_bytes = id.to_le_bytes();
    nonce_bytes[..8].copy_from_slice(&msg_id_bytes);
    nonce_bytes[8..].copy_from_slice(&[0u8; 4]);

    let nonce = GenericArray::from_slice(&nonce_bytes);

    aead.decrypt(&nonce, ciphertext)
        .expect("Failed to decrypt an object!")
}
