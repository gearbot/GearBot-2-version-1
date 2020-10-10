use aes_gcm::aead::generic_array::GenericArray;
use chrono::{DateTime, Utc};
use tokio::sync::mpsc::UnboundedSender;
use twilight_gateway::Cluster;
use twilight_http::Client as HttpClient;
use twilight_model::{
    channel::Message,
    id::{GuildId, UserId},
    user::CurrentUser,
};

mod cache;
mod cold_resume;
mod database;
mod logpump;
mod permissions;
mod stats;
pub(crate) mod status;

pub use stats::BotStats;

use crate::core::cache::Cache;
use crate::core::GuildConfig;
use crate::crypto::EncryptionKey;
use crate::database::api_structs::{RawTeamMembers, TeamInfo, TeamMember};
use crate::database::Redis;
use crate::translation::{GearBotString, Translations};
use crate::utils::LogType;
use crate::SchemeInfo;
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use unic_langid::LanguageIdentifier;

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
    configs: RwLock<HashMap<GuildId, Arc<GuildConfig>>>,
    pub backing_database: sqlx::PgPool,
    pub translations: Translations,
    __main_encryption_key: Option<Vec<u8>>,
    log_pumps: RwLock<HashMap<GuildId, UnboundedSender<(DateTime<Utc>, LogType)>>>,
    pub redis_cache: Redis,
    pub scheme_info: SchemeInfo,
    pub shard_states: RwLock<HashMap<u64, ShardState>>,
    pub start_time: DateTime<Utc>,
    pub global_admins: Vec<UserId>,
    team_info: RawTeamMembers,
}

impl BotContext {
    pub fn new(
        bot_core: (Cache, Cluster, SchemeInfo),
        http_info: (HttpClient, CurrentUser),
        databases: (sqlx::PgPool, Redis),
        translations: Translations,
        config_ops: (Option<Vec<u8>>, Vec<u64>),
        stats: Arc<BotStats>,
    ) -> Self {
        let scheme_info = bot_core.2;
        let mut shard_states = HashMap::with_capacity(scheme_info.shards_per_cluster as usize);
        for i in scheme_info.cluster_id * scheme_info.shards_per_cluster
            ..scheme_info.cluster_id * scheme_info.shards_per_cluster + scheme_info.shards_per_cluster
        {
            shard_states.insert(i, ShardState::PendingCreation);
            bot_core
                .0
                .missing_per_shard
                .write()
                .expect("Global shard state tracking got poisoned!")
                .insert(i, AtomicU64::new(0));
        }

        let global_admins = config_ops.1.into_iter().map(UserId).collect();

        stats.shard_counts.pending.set(scheme_info.shards_per_cluster as i64);

        let team_info: RawTeamMembers =
            toml::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/team.toml"))).unwrap();

        BotContext {
            cache: bot_core.0,
            cluster: bot_core.1,
            http: http_info.0,
            stats,
            status_type: RwLock::new(3),
            status_text: RwLock::new(String::from("the commands turn")),
            bot_user: http_info.1,
            configs: RwLock::new(HashMap::new()),
            backing_database: databases.0,
            translations,
            __main_encryption_key: config_ops.0,
            log_pumps: RwLock::new(HashMap::new()),
            redis_cache: databases.1,
            scheme_info,
            shard_states: RwLock::new(shard_states),
            start_time: Utc::now(),
            global_admins,
            team_info,
        }
    }

    /// Returns if a message was sent by us.
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }

    fn __get_main_encryption_key(&self) -> &EncryptionKey {
        if let Some(mk_bytes) = &self.__main_encryption_key {
            GenericArray::from_slice(mk_bytes)
        } else {
            // It will always be returned, but the other location it could come from
            // is not implemented as of yet.
            unreachable!()
        }
    }

    pub fn translate(&self, language: &LanguageIdentifier, key: GearBotString) -> String {
        self.translations.get_text_plain(language, key).to_string()
    }

    pub fn translate_with_args(
        &self,
        language: &LanguageIdentifier,
        string_key: GearBotString,
        args: &FluentArgs<'_>,
    ) -> String {
        self.translations
            .get_text_with_args(language, string_key, args)
            .replace("\\n", "\n")
    }

    pub async fn get_team_info(&self) -> TeamInfo {
        let mut members = vec![];
        for m in &self.team_info.members {
            let user = self.get_user(UserId(m.id)).await.unwrap();
            members.push(TeamMember {
                username: user.username.clone(),
                discriminator: user.discriminator.clone(),
                id: m.id,
                avatar: user.avatar.clone().unwrap_or("".to_string()),
                team: m.team.clone(),
                socials: m.socials.clone(),
            });
        }

        TeamInfo { members }
    }
}
