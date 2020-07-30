use aes_gcm::aead::generic_array::GenericArray;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use tokio::sync::{mpsc::UnboundedSender, RwLock};
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::{
    channel::Message,
    id::{GuildId, UserId},
    user::CurrentUser,
};

pub use stats::BotStats;

use crate::core::cache::Cache;
use crate::core::GuildConfig;
use crate::crypto::EncryptionKey;
use crate::translation::Translations;
use crate::utils::LogType;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

#[derive(PartialEq, Debug)]
pub enum ShardState {
    PendingCreation,
    Connecting,
    Identifying,
    Connected,
    Ready,
    Resuming,
    Reconnecting,
    Disconnected,
}

pub struct BotContext {
    pub cache: Cache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: Arc<BotStats>,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
    pub bot_user: CurrentUser,
    configs: DashMap<GuildId, GuildConfig>,
    pub pool: sqlx::PgPool,
    pub translations: Translations,
    __static_master_key: Option<Vec<u8>>,
    log_pumps: DashMap<GuildId, UnboundedSender<(DateTime<Utc>, LogType)>>,
    pub redis_pool: darkredis::ConnectionPool,
    pub cluster_id: u64,
    pub shards_per_cluster: u64,
    pub total_shards: u64,
    pub shard_states: DashMap<u64, ShardState>,
    pub start_time: DateTime<Utc>,
    pub global_admins: Vec<UserId>,
}

impl BotContext {
    pub fn new(
        cache: Cache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: sqlx::PgPool,
        translations: Translations,
        key: Option<Vec<u8>>,
        redis_pool: darkredis::ConnectionPool,
        cluster_id: u64,
        shards_per_cluster: u64,
        total_shards: u64,
        stats: Arc<BotStats>,
        global_admins: Vec<u64>,
    ) -> Self {
        let shard_states = DashMap::with_capacity(shards_per_cluster as usize);
        for i in cluster_id * shards_per_cluster..cluster_id * shards_per_cluster + shards_per_cluster {
            shard_states.insert(i, ShardState::PendingCreation);
            cache
                .missing_per_shard
                .write()
                .expect("Global shard state tracking got poisoned!")
                .insert(i, AtomicU64::new(0));
        }

        let global_admins = global_admins.into_iter().map(|id| UserId(id)).collect();

        stats.shard_counts.pending.set(shards_per_cluster as i64);
        BotContext {
            cache,
            cluster,
            http,
            stats,
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
            shard_states,
            start_time: Utc::now(),
            global_admins,
        }
    }

    /// Returns if a message was sent by us.
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }

    fn __get_master_key(&self) -> &EncryptionKey {
        if let Some(mk_bytes) = &self.__static_master_key {
            GenericArray::from_slice(mk_bytes)
        } else {
            // It will always be returned, but the other location it could come from
            // is not implemented as of yet.
            unreachable!()
        }
    }
}

mod cache;

mod database;
mod logpump;

mod cold_resume;
mod stats;
pub(crate) mod status;
