use std::str::FromStr;

use aes_gcm::aead::generic_array::{typenum::U32, GenericArray};
use deadpool_postgres::{Manager, Pool};
use git_version::git_version;
use log::{debug, info};
use tokio_postgres::{Config, NoTls};
use twilight::http::{
    request::channel::message::allowed_mentions::AllowedMentionsBuilder, Client as HttpClient,
};

use crate::core::{gearbot, logging, BotConfig, ColdRebootData};
use crate::database::migrations::embedded;

mod commands;
mod core;
mod database;
mod parser;

mod utils;
use clap::{App, Arg};
use darkredis::{CommandList, ConnectionPool, Value};
use futures::TryStreamExt;
use utils::Error;

mod translation;
use crate::core::gearbot::GearBot;
use std::time::Duration;
use tokio::runtime::Runtime;
use translation::load_translations;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_VERSION: &str = git_version!();

pub type CommandResult = Result<(), Error>;

pub type EncryptionKey = GenericArray<u8, U32>;

fn main() -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    runtime.block_on(async move { real_main().await })?;

    runtime.shutdown_timeout(Duration::from_secs(90));
    Ok(())
}

async fn real_main() -> Result<(), Error> {
    //parse CLI args
    let args = App::new("GearBot")
        .arg(Arg::with_name("cluster"))
        .arg(Arg::with_name("shards_per_cluster"))
        .arg(Arg::with_name("total_shards"))
        .get_matches();

    if let Err(e) = logging::initialize() {
        gearbot_error!("{}", e);
        return Err(e);
    }

    info!("Gearbot v{} starting!", VERSION);
    // Read config file
    let config = BotConfig::new("config.toml")?;
    debug!("Loaded config file");

    if config.__master_key.is_none() {
        panic!("The KMS needs built before GearBot can work without a static master key!");
    }

    let mut builder = HttpClient::builder();
    builder.token(&config.tokens.discord);

    builder.default_allowed_mentions(AllowedMentionsBuilder::new().build_solo());

    let http = builder.build()?;
    // Validate token and figure out who we are
    let user = http.current_user().await?;
    info!(
        "Token validated, connecting to discord as {}#{}",
        user.name, user.discriminator
    );

    logging::initialize_discord_webhooks(http.clone(), &config, user.clone());

    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", VERSION);

    let translations = load_translations();
    gearbot_info!("Loaded translations!");

    //connect to the database
    let manager = Manager::new(Config::from_str(&config.database.postgres)?, NoTls);
    let postgres_pool = Pool::new(manager, 10);
    let mut connection = postgres_pool.get().await?;

    info!("Connected to postgres!");

    let redis_pool = ConnectionPool::create(config.database.redis.clone(), None, 5)
        .await
        .unwrap();
    info!("Connected to redis!");

    gearbot_info!("Database connections established");

    //TODO: wrap this
    embedded::migrations::runner()
        .run_async(&mut **connection)
        .await
        .map_err(|e| Error::DatabaseMigration(e.to_string()))?;

    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled

    let cluster = args
        .value_of("cluster")
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    let shards_per_cluster = args
        .value_of("shards_per_cluster")
        .unwrap_or("1")
        .parse::<u64>()
        .unwrap_or(1);
    let total_shards = args
        .value_of("total_shards")
        .unwrap_or("1")
        .parse::<u64>()
        .unwrap_or(1);

    if let Err(e) = GearBot::run(
        cluster,
        shards_per_cluster,
        total_shards,
        config,
        http,
        user,
        postgres_pool,
        redis_pool,
        translations,
    )
    .await
    {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    Ok(())
}
