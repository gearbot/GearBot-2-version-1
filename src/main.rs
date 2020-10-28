// TODO: Remove this when the bot is a bit more functional
#![allow(dead_code)]

use std::time::Duration;

use git_version::git_version;
use log::info;
use tokio::runtime::Runtime;
use twilight_http::{
    client::Proxy, request::channel::message::allowed_mentions::AllowedMentionsBuilder, Client as HttpClient,
};

use commands::ROOT_NODE;
use translation::load_translations;

use crate::core::gearbot;
use crate::core::{logging, BotConfig};
use crate::error::StartupError;
use std::env;

mod commands;
mod core;
mod crypto;
mod database;
mod error;
mod parser;

mod translation;
mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_VERSION: &str = git_version!();

#[derive(Debug, Copy, Clone)]
pub struct SchemeInfo {
    pub cluster_id: u64,
    pub shards_per_cluster: u64,
    pub total_shards: u64,
}

fn main() -> Result<(), StartupError> {
    let mut runtime = Runtime::new()?;

    runtime.block_on(async move { real_main().await })?;

    runtime.shutdown_timeout(Duration::from_secs(90));
    Ok(())
}

async fn real_main() -> Result<(), StartupError> {
    println!("Gearbot v{} starting!", VERSION);
    // Read config file
    let config = BotConfig::new(&env::var("CONFIG_FILE").unwrap_or("config.toml".to_string()))?;
    println!("Loaded config file");

    let mut builder = HttpClient::builder()
        .token(&config.tokens.discord)
        .default_allowed_mentions(AllowedMentionsBuilder::new().build_solo());
    if let Some(proxy_url) = &config.proxy_url {
        builder = builder
            .proxy(Proxy::all(proxy_url).unwrap())
            .proxy_http(true)
            .ratelimiter(None);
    }

    let http = builder.build()?;
    // Validate token and figure out who we are
    let bot_user = http.current_user().await?;
    info!(
        "Token validated, connecting to discord as {}#{}",
        bot_user.name, bot_user.discriminator
    );

    if let Err(e) = logging::initialize(http.clone(), &config, bot_user.clone()) {
        gearbot_error!("{}", e);
        return Err(e);
    }

    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", VERSION);

    let translations = load_translations();
    gearbot_info!("Loaded translations!");

    let datastore = database::DataStorage::initalize(&config).await?;

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

    if let Err(e) = gearbot::run(scheme_info, config, http, bot_user, datastore, translations).await {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    Ok(())
}
