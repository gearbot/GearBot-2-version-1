use std::str::FromStr;

use deadpool_postgres::{Manager, Pool};
use tokio_postgres::{Config, NoTls};
use twilight::http::Client as HttpClient;

use git_version::git_version;
use utils::errors::Error;

use crate::core::BotConfig;
use crate::core::GearBot;
use crate::core::logging;
use crate::database::migrations::embedded;

mod core;
mod gears;
mod utils;
mod database;
mod parser;

pub type CommandResult = Result<(), Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Read config file
    let config = BotConfig::new("config.toml")?;

    let http = HttpClient::new(&config.tokens.discord);

    // Initialize logger
    // Cloning http here is fine because this instance is only used for calling our global log webhook
    // and the rate limits on that are completely separate from all other rate limits
    if let Err(e) = logging::initialize(http.clone(), &config) {
        gearbot_error!("{}", e);
        return Err(e);
    }

    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", git_version!());

    //connect to the database
    let manager = Manager::new(Config::from_str(&config.database.postgres)?, NoTls);
    let pool = Pool::new(manager, 10);
    let mut connection = pool.get().await?;

    gearbot_info!("Connected to the database!");

    //TODO: wrap this
    embedded::migrations::runner().run_async(&mut **connection).await.map_err(|e| Error::DatabaseMigrationError(e.to_string()))?;


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
