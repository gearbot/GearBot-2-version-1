use std::error;
use std::sync::Arc;

use log::{error, info};
use tokio::stream::StreamExt;
use twilight::{
    cache::{
        InMemoryCache,
        twilight_cache_inmemory::config::{EventType as CacheEventType, InMemoryConfigBuilder},
    },
    command_parser::{CommandParserConfig, Parser},
    gateway::{cluster::{Cluster, ClusterConfig, config::ShardScheme}, shard::Event},
    http::Client as HttpClient,
    model::gateway::GatewayIntents,
};

use crate::core::BotConfig;
use crate::core::GearBot;
use crate::core::logging;

mod core;
mod gears;

pub enum Error {
    SomethingBadHappened,
    MissingToken,
}

pub type CommandResult = Result<(), Error>;

pub const COMMAND_LIST: [&str; 3] = [
    "about",
    "ping",
    "echo",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error + Send + Sync>> {
    // read config file
    let config = BotConfig::new("config.toml")?;
    let http = HttpClient::new(&config.tokens.discord);

    //initialize logger
    // cloning http here is fine because this instance is only used for calling our global log webhook
    // and the rate limits on that are completely separate from all other rate limits
    logging::initialize(http.clone(), &config);
    gearbot_important!("Starting Gearbot. Hello there, Ferris!");

    //generate command list


    let gearbot = GearBot::run(config, http).await?;


    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled



    // In the future, this will need to be a RwLock when there is a database, etc




    Ok(())
}


