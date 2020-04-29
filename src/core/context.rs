use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use twilight::cache::InMemoryCache;
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::channel::Message;

use crate::core::GuildConfig;
use crate::gearbot_error;
use crate::utils::Error;
use dashmap::mapref::one::Ref;
use dashmap::DashMap;
use deadpool_postgres::{Pool, PoolError};
use git_version::git_version;
use log::debug;
use postgres_types::Type;
use serde_json;
use std::borrow::Borrow;
use std::sync::Arc;
use twilight::model::user::CurrentUser;

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
    configs: DashMap<i64, GuildConfig>,
    pool: Pool,
}

impl Context {
    pub fn new(
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: Pool,
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
        }
    }

    pub async fn get_config(&self, guild_id: i64) -> Result<Ref<'_, i64, GuildConfig>, Error> {
        debug!("getting config for {}", guild_id);
        match self.configs.get(&guild_id) {
            Some(config) => Ok(config),
            None => {
                debug!("fetching from db");
                let client = self.pool.get().await?;
                let statement = client
                    .prepare_typed("SELECT config from guildconfig where id=$1", &[Type::INT8])
                    .await?;
                let rows = client.query(&statement, &[&guild_id]).await?;
                let config;
                if rows.len() == 0 {
                    debug!("none found in db");
                    config = GuildConfig::new();
                    tokio::spawn(async move {
                        debug!("inserting blank one into database");
                        let statement = client
                            .prepare_typed(
                                "INSERT INTO guildconfig (id, config) VALUES ($1, $2)",
                                &[Type::INT8, Type::JSON],
                            )
                            .await
                            .unwrap();
                        if let Err(e) = client
                            .execute(
                                &statement,
                                &[
                                    &guild_id,
                                    &serde_json::to_value(&GuildConfig::new()).unwrap(),
                                ],
                            )
                            .await
                        {
                            gearbot_error!("{}", e);
                        };
                        debug!("inserted");
                    });
                } else {
                    debug!("found one in the db");
                    config = serde_json::from_str(rows[0].get(0))?;
                }
                self.configs.insert(guild_id, config);
                Ok(self.configs.get(&guild_id).unwrap())
            }
        }
    }

    /// Returns if a message was sent by us.
    ///
    /// Returns None if we couldn't currently get a lock on the cache, but
    /// rarely, if ever should this happen.
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }
}
