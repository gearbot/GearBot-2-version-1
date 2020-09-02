// TODO: Remove this when the bot is a bit more functional
#![allow(dead_code)]

use std::time::Duration;

use git_version::git_version;
use log::{debug, info};
use tokio::runtime::Runtime;
use twilight::http::{request::channel::message::allowed_mentions::AllowedMentionsBuilder, Client as HttpClient};

use commands::ROOT_NODE;
use translation::load_translations;
use utils::Error;

use crate::core::gearbot;
use crate::core::{logging, BotConfig};

mod commands;
mod core;
mod crypto;
mod database;
mod parser;

mod translation;
mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_VERSION: &str = git_version!();

pub type CommandResult = Result<(), Error>;

#[derive(Debug, Copy, Clone)]
pub struct SchemeInfo {
    pub cluster_id: u64,
    pub shards_per_cluster: u64,
    pub total_shards: u64,
}

fn main() -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    runtime.block_on(async move { real_main().await })?;

    runtime.shutdown_timeout(Duration::from_secs(90));
    Ok(())
}

async fn real_main() -> Result<(), Error> {
    if let Err(e) = logging::initialize() {
        gearbot_error!("{}", e);
        return Err(e);
    }

    info!("Gearbot v{} starting!", VERSION);
    // Read config file
    let config = BotConfig::new("config.toml")?;
    debug!("Loaded config file");

    if config.__main_encryption_key.is_none() {
        panic!("The KMS needs built before GearBot can work without a static main encryption key!");
    }

    let mut builder = HttpClient::builder();
    builder = builder.token(&config.tokens.discord);

    builder = builder.default_allowed_mentions(AllowedMentionsBuilder::new().build_solo());

    let http = builder.clone().build()?;
    // Validate token and figure out who we are
    let user = http.current_user().await?;
    info!(
        "Token validated, connecting to discord as {}#{}",
        user.name, user.discriminator
    );

    logging::initialize_discord_webhooks(builder.build()?, &config, user.clone());

    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", VERSION);

    let translations = load_translations();
    gearbot_info!("Loaded translations!");

    //connect to the database
    let postgres_pool = sqlx::Pool::connect(&config.database.postgres).await?;

    info!("Connected to postgres!");

    info!("Handling database migrations...");
    sqlx::migrate!("./migrations")
        .run(&postgres_pool)
        .await
        .expect("Failed to run database migrations!");

    info!("Finished migrations!");

    let redis_pool = match darkredis::ConnectionPool::create(config.database.redis.clone(), None, 5).await {
        Ok(pool) => pool,
        Err(e) => {
            gearbot_error!("Failed to connect to the redis database! {}", e);
            return Err(Error::RedisError(e));
        }
    };

    info!("Connected to redis!");

    gearbot_info!("Database connections established");

    {
        info!("Populating command list");
        ROOT_NODE.all_commands.get("something");
        info!("Command list populated")
    }

    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled

    // Parse CLI arguments for sharding and cluster info
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let cluster_id = args
        .get(0)
        .map(|cs| cs.parse::<u64>().unwrap_or_default())
        .unwrap_or_default();
    let shards_per_cluster = args.get(1).map(|spc| spc.parse::<u64>().unwrap_or(1)).unwrap_or(1);
    let total_shards = args.get(2).map(|ts| ts.parse::<u64>().unwrap_or(1)).unwrap_or(1);

    let scheme_info = SchemeInfo {
        cluster_id,
        shards_per_cluster,
        total_shards,
    };

    if let Err(e) = gearbot::run(scheme_info, config, http, user, postgres_pool, redis_pool, translations).await {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    Ok(())
}
