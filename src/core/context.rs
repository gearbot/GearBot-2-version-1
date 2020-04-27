use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use twilight::cache::InMemoryCache;
use twilight::gateway::Cluster;
use twilight::http::Client as HttpClient;
use twilight::model::channel::Message;

use git_version::git_version;

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
            if let Some(is_own) = ctx.is_own(msg).await {
                if is_own {
                    ctx.stats.my_messages.fetch_add(1, Ordering::Relaxed);
                }
                ctx.stats.bot_messages.fetch_add(1, Ordering::Relaxed);
            }
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

// In the future, any database handles or anything that holds real state will need
// put behind a `RwLock`.
#[derive(Debug)]
pub struct Context {
    pub cache: InMemoryCache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: BotStats,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
}

impl Context {
    pub fn new(cache: InMemoryCache, cluster: Cluster, http: HttpClient) -> Self {
        Context {
            cache,
            cluster,
            http,
            stats: BotStats::default(),
            status_type: RwLock::new(3),
            status_text: RwLock::new(String::from("the commands turn")),
        }
    }

    /// Returns if a message was sent by us.
    ///
    /// Returns None if we couldn't currently get a lock on the cache, but
    /// rarely, if ever should this happen.
    pub async fn is_own(&self, other: &Message) -> Option<bool> {
        // This will always exist when called.
        let bot = self.cache.current_user().await.unwrap();

        if let Some(bot) = bot {
            Some(bot.id == other.author.id)
        } else {
            None
        }
    }
}
