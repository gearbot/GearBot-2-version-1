use aes_gcm::aead::generic_array::GenericArray;
use chrono::{DateTime, Utc};
use darkredis::ConnectionPool;
use dashmap::DashMap;
use deadpool_postgres::Pool;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::{channel::Message, id::GuildId, user::CurrentUser};

pub use stats::BotStats;

use crate::core::cache::Cache;
use crate::core::GuildConfig;
use crate::translation::Translations;
use crate::utils::LogType;
use crate::EncryptionKey;

pub struct BotContext {
    pub cache: Cache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: BotStats,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
    pub bot_user: CurrentUser,
    configs: DashMap<GuildId, GuildConfig>,
    pub pool: Pool,
    pub translations: Translations,
    __static_master_key: Option<Vec<u8>>,
    log_pumps: DashMap<GuildId, UnboundedSender<(DateTime<Utc>, LogType)>>,
    pub redis_pool: ConnectionPool,
    pub cluster_id: u64,
    pub shards_per_cluster: u64,
    pub total_shards: u64,
}

impl BotContext {
    pub fn new(
        cache: Cache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: Pool,
        translations: Translations,
        key: Option<Vec<u8>>,
        redis_pool: ConnectionPool,
        cluster_id: u64,
        shards_per_cluster: u64,
        total_shards: u64,
    ) -> Self {
        BotContext {
            cache,
            cluster,
            http,
            stats: BotStats::default(),
            status_type: RwLock::new(3),
            status_text: RwLock::new(String::from("the commands turn")),
            bot_user,
            configs: DashMap::new(),
            pool,
            translations,
            __static_master_key: key,
            log_pumps: DashMap::new(),
            redis_pool,
            cluster_id,
            shards_per_cluster,
            total_shards,
        }
    }

    /// Returns if a message was sent by us.
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }

    fn __get_master_key(&self) -> Option<&EncryptionKey> {
        if let Some(mk_bytes) = &self.__static_master_key {
            let key = GenericArray::from_slice(mk_bytes);
            Some(key)
        } else {
            None
        }
    }
}

mod cache;

mod database;
mod logpump;

mod cold_resume;
mod stats;
mod status;
