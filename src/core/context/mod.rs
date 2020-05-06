use crate::core::context::stats::BotStats;
use crate::core::GuildConfig;
use crate::EncryptionKey;
use aes_gcm::aead::generic_array::GenericArray;
use dashmap::DashMap;
use deadpool_postgres::Pool;
use futures::channel::oneshot::Sender;
use git_version::git_version;
use std::sync::RwLock;
use twilight::cache::InMemoryCache;
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::channel::Message;
use twilight::model::gateway::payload::MemberChunk;
use twilight::model::id::GuildId;
use twilight::model::user::CurrentUser;

const GIT_VERSION: &str = git_version!();

pub struct Context {
    pub cache: InMemoryCache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: BotStats,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
    pub bot_user: CurrentUser,
    configs: DashMap<GuildId, GuildConfig>,
    pool: Pool,
    __static_master_key: Option<Vec<u8>>,
    pub chunk_requests: DashMap<String, Sender<MemberChunk>>,
}

impl Context {
    pub fn new(
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: Pool,
        key: Option<Vec<u8>>,
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
            __static_master_key: key,
            chunk_requests: DashMap::new(),
        }
    }

    /// Returns if a message was sent by us.
    ///
    /// Returns None if we couldn't currently get a lock on the cache, but
    /// rarely, if ever should this happen.
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
mod stats;
