use std::str::FromStr;

use deadpool_postgres::{Manager, Pool};
use log::{debug, info};
use tokio_postgres::{Config, NoTls};
use twilight::http::Client as HttpClient;

use git_version::git_version;
use utils::Error;

use crate::core::BotConfig;
use crate::core::GearBot;
use crate::core::logging;
use crate::database::migrations::embedded;

mod commands;
mod core;
mod database;
mod parser;
mod utils;

pub static VERSION: &str = env!("CARGO_PKG_VERSION");
pub static GIT_VERSION: &str = git_version!();

pub type CommandResult = Result<(), Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    if let Err(e) = logging::initialize() {
        gearbot_error!("{}", e);
        return Err(e);
    }

    info!("Gearbot v{} starting!", VERSION);
    // Read config file
    let config = BotConfig::new("config.toml")?;
    debug!("Loaded config file");
    let http = HttpClient::new(&config.tokens.discord);
    //validate token and figure out who we are
    let user = http.current_user().await?;
    info!("Token validated, connecting to discord as {}#{}", user.name, user.discriminator);
    logging::initialize_discord_webhooks(http.clone(), &config, user);



    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", VERSION);
    gearbot_error!("test error");
    gearbot_warn!("test warning");

    //connect to the database
    let manager = Manager::new(Config::from_str(&config.database.postgres)?, NoTls);
    let pool = Pool::new(manager, 10);
    let mut connection = pool.get().await?;

    gearbot_info!("Connected to the database!");

    //TODO: wrap this
    embedded::migrations::runner()
        .run_async(&mut **connection)
        .await
        .map_err(|e| Error::DatabaseMigrationError(e.to_string()))?;

    // tokio::spawn(async move {
    //     if let Err(e) = connection.await {
    //         gearbot_error!("connection error: {}", e);
    //     }
    // });

    //generate command list

    if let Err(e) = GearBot::run(&config, http).await {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled

    Ok(())
}
