use std::sync::atomic::{AtomicUsize, Ordering};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, NewAead},
    Aes256Gcm,
};

use chrono::{DateTime, Utc};
use rand::{thread_rng, RngCore};
use tokio::sync::RwLock;
use twilight::cache::InMemoryCache;
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::channel::Message;
use twilight::model::{
    channel::message::MessageType,
    id::{ChannelId, GuildId, MessageId, UserId},
    user::CurrentUser,
};

use crate::utils::{Error, FetchError};
use crate::{core::GuildConfig, EncryptionKey};
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use deadpool_postgres::Pool;
use git_version::git_version;
use log::info;
use postgres_types::Type;
use serde_json;

const GIT_VERSION: &str = git_version!();

#[derive(Debug)]
pub struct BotStats {
    pub start_time: DateTime<Utc>,
    pub user_messages: AtomicUsize,
    pub bot_messages: AtomicUsize,
    pub my_messages: AtomicUsize,
    pub error_count: AtomicUsize,
    pub commands_ran: AtomicUsize,
    pub custom_commands_ran: AtomicUsize,
    pub guilds: AtomicUsize,
    pub version: &'static str,
}

impl BotStats {
    pub async fn new_message(&self, ctx: &Context, msg: &Message) {
        if msg.author.bot {
            // This will simply skip incrementing it if we couldn't get
            // a lock on the cache. No harm done.
            if ctx.is_own(msg) {
                ctx.stats.my_messages.fetch_add(1, Ordering::Relaxed);
            }
            ctx.stats.bot_messages.fetch_add(1, Ordering::Relaxed);
        } else {
            ctx.stats.user_messages.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub async fn had_error(&self) {
        self.error_count.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn new_guild(&self) {
        self.guilds.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn left_guild(&self) {
        self.guilds.fetch_sub(1, Ordering::Relaxed);
    }

    pub async fn command_used(&self, is_custom: bool) {
        if !is_custom {
            self.commands_ran.fetch_add(1, Ordering::Relaxed);
        } else {
            self.custom_commands_ran.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Default for BotStats {
    fn default() -> Self {
        BotStats {
            start_time: Utc::now(),
            user_messages: AtomicUsize::new(0),
            bot_messages: AtomicUsize::new(0),
            my_messages: AtomicUsize::new(0),
            error_count: AtomicUsize::new(0),
            commands_ran: AtomicUsize::new(0),
            custom_commands_ran: AtomicUsize::new(0),
            guilds: AtomicUsize::new(0),
            version: GIT_VERSION,
        }
    }
}

#[derive(Debug)]
pub struct LoadingState {
    to_load: u32,
    loaded: u32,
}
// In the future, any database handles or anything that holds real state will need
// put behind a `RwLock`.
pub struct Context {
    pub cache: InMemoryCache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: BotStats,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
    pub bot_user: CurrentUser,
    configs: DashMap<GuildId, GuildConfig>,
    __static_master_key: Option<Vec<u8>>,
    pool: Pool,
}

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
    pub fn new(
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: Pool,
        static_key: Option<Vec<u8>>,
    ) -> Self {
        Context {
            cache,
            cluster,
            http,
            stats: BotStats::default(),
            status_type: RwLock::new(3),
            status_text: RwLock::new(String::from("the commands turn")),
            bot_user,
            configs: DashMap::new(),
            pool,
            __static_master_key: static_key,
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

            info!(
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

    pub async fn insert_user_message(&self, msg: &Message, guild_id: GuildId) -> Result<(), Error> {
        // All guilds need to have a config before anything can happen thanks to encryption.
        let _ = self.get_config(guild_id).await?;
        let client = self.pool.get().await?;

        let msg_id = msg.id.0 as i64;
        let msg_type = msg.kind as i16;
        let author_id = msg.author.id.0 as i64;
        let channel_id = msg.channel_id.0 as i64;
        let pinned = msg.pinned;

        let start = std::time::Instant::now();
        let ciphertext = {
            let plaintext = msg.content.as_bytes();
            let guild_key = self.get_guild_encryption_key(guild_id).await?;

            encrypt_bytes(plaintext, &guild_key, msg_id as u64)
        };

        let finish_crypto = std::time::Instant::now();

        info!(
            "It took {}ms to encrypt the user message!",
            (finish_crypto - start).as_millis()
        );

        let guild_id = guild_id.0 as i64;
        let statement = client
            .prepare_typed(
                "INSERT INTO message (id, content, author_id, channel_id, guild_id, type, pinned)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    Type::INT8,
                    Type::BYTEA,
                    Type::INT8,
                    Type::INT8,
                    Type::INT8,
                    Type::INT2,
                    Type::BOOL,
                ],
            )
            .await?;

        client
            .execute(
                &statement,
                &[
                    &msg_id,
                    &ciphertext,
                    &author_id,
                    &channel_id,
                    &guild_id,
                    &msg_type,
                    &pinned,
                ],
            )
            .await?;

        info!("Logged a user message!");

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
                    let encrypted_guild_key = {
                        let mut csprng = thread_rng();
                        // Each guild has its own encryption key. This allows us, in the event of a compromise of the master key,
                        // to simply re-encrypt the guild keys instead of millions of messages.
                        let mut guild_encryption_key = [0u8; 32];
                        csprng.fill_bytes(&mut guild_encryption_key);

                        let master_key = self.__get_master_key().unwrap();
                        encrypt_bytes(&guild_encryption_key, master_key, guild_id.0 as u64)
                    };

                    let config = GuildConfig::default();
                    info!("No config found for {}, inserting blank one", guild_id);
                    let statement = client
                        .prepare_typed(
                            "INSERT INTO guildconfig (id, config, encryption_key) VALUES ($1, $2, $3)",
                            &[Type::INT8, Type::JSON, Type::BYTEA],
                        )
                        .await?;
                    client
                        .execute(
                            &statement,
                            &[
                                &(guild_id.0 as i64),
                                &serde_json::to_value(&GuildConfig::default()).unwrap(),
                                &encrypted_guild_key,
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

    /// Returns if a message was sent by us
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }

    /// Returns the master key that is used to encrypt and decrypt guild keys.
    fn __get_master_key(&self) -> Option<&EncryptionKey> {
        if let Some(mk_bytes) = &self.__static_master_key {
            let key = GenericArray::from_slice(mk_bytes);
            Some(key)
        } else {
            None
        }
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
