use std::{error, fmt, io};

use deadpool_postgres::PoolError;
use twilight::gateway::cluster;
use twilight::http;

#[derive(Debug)]
pub enum Error {
    CmdError(CommandError),
    InvalidSession,
    MissingToken,
    NoConfig,
    InvalidConfig,
    InvalidLoggingWebhook(String),
    NoLoggingSpec,
    IoError(io::Error),
    TwilightHttp(http::Error),
    TwilightCluster(cluster::Error),
    DatabaseError(tokio_postgres::error::Error),
    PoolError(PoolError),
    DatabaseMigrationError(String)
}

#[derive(Debug)]
pub enum CommandError {
    WrongArgCount {
        expected: u8,
        provided: u8,
    }
}

impl error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::WrongArgCount { expected, provided } => {
                if expected > provided {
                    write!(f, "Too many arguments were provided! Expected {}, but found {}", expected, provided)
                } else {
                    write!(f, "Not enough arguments were provided! Expected {}, but found {}", expected, provided)
                }
            }
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CmdError(e) => write!(f, "{}", e),
            Error::InvalidSession => write!(f, "The gateway invalidated our session unrecoverably!"),
            // For errors that actually happen during runtime, we can have the logging macros here too
            Error::MissingToken => write!(f, "The bot was missing its token, unable to start!"),
            Error::NoConfig => write!(f, "The config file couldn't be found, unable to start!"),
            Error::InvalidConfig => write!(f, "The config file was not in the correct format!"),
            Error::InvalidLoggingWebhook(wurl) => write!(f, "The webhook URL {} was invalid", wurl),
            Error::NoLoggingSpec => write!(f, "The logging configuration couldn't be found!"),
            Error::IoError(e) => write!(f, "An IO error occurred during a task: {}", e),
            Error::TwilightHttp(e) => write!(f, "An error occurred making a Discord request: {}", e),
            Error::TwilightCluster(e) => write!(f, "An error occurred on a cluster request: {}", e),
            Error::DatabaseError(e) => {write!(f, "A database error occurred: {}", e)}
            Error::PoolError(e) => {write!(f, "An error occurred in the database pool: {}", e)}
            Error::DatabaseMigrationError(e) => {write!(f, "Failed to migrate the database: {}", e)}
        }
    }
}

impl From<CommandError> for Error {
    fn from(e: CommandError) -> Self {
        Error::CmdError(e)
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

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Self { Error::DatabaseError(e) }
}

impl From<PoolError> for Error {
    fn from(e: PoolError) -> Self {Error::PoolError(e)}
}
