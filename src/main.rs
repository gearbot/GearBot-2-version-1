use std::{error, fmt, io};

use twilight::gateway::cluster;
use twilight::{http, http::Client as HttpClient};

use git_version::git_version;

use crate::core::logging;
use crate::core::BotConfig;
use crate::core::GearBot;

mod core;
mod gears;

#[derive(Debug)]
pub enum Error {
    InvalidSession,
    MissingToken,
    NoConfig,
    InvalidConfig,
    InvalidLoggingWebhook(String),
    NoLoggingSpec,
    IoError(io::Error),
    TwilightHttp(http::Error),
    TwilightCluster(cluster::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidSession => {
                write!(f, "The gateway invalidated our session unrecoverably!")
            }
            // For errors that actually happen during runtime, we can have the logging macros here too
            Error::MissingToken => write!(f, "The bot was missing its token, unable to start!"),
            Error::NoConfig => write!(f, "The config file couldn't be found, unable to start!"),
            Error::InvalidConfig => write!(f, "The config file was not in the correct format!"),
            Error::InvalidLoggingWebhook(wurl) => write!(f, "The webhook URL {} was invalid", wurl),
            Error::NoLoggingSpec => write!(f, "The logging configuration couldn't be found!"),
            Error::IoError(e) => write!(f, "An IO error occured during a task: {}", e),
            Error::TwilightHttp(e) => write!(f, "An error occured making a Discord request: {}", e),
            Error::TwilightCluster(e) => write!(f, "An error occured on a cluster request: {}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<http::Error> for Error {
    fn from(e: http::Error) -> Self {
        Error::TwilightHttp(e)
    }
}

impl From<cluster::Error> for Error {
    fn from(e: cluster::Error) -> Self {
        Error::TwilightCluster(e)
    }
}

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

    //generate command list

    if let Err(e) = GearBot::run(&config, http).await {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled

    Ok(())
}
