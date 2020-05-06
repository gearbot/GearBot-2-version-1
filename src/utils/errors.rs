use std::{error, fmt, io};

use deadpool_postgres::PoolError;
use serde::export::Formatter;
use twilight::cache::twilight_cache_inmemory;
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
    // This error will never occur according to the Cache docs, as it exists solely to
    // fullfill a trait API.
    Cache(twilight_cache_inmemory::InMemoryCacheError),
    Database(tokio_postgres::error::Error),
    DatabaseAction(FetchError),
    Pool(PoolError),
    DatabaseMigration(String),
    UnknownEmoji(String),
    Serde(serde_json::error::Error),
    ParseError(ParseError),
}

#[derive(Debug)]
pub enum CommandError {
    // WrongArgCount { expected: u8, provided: u8 },
    NoDM,
}

#[derive(Debug)]
pub enum ParseError {
    MissingArgument,
    MemberNotFoundById(u64),
    MemberNotFoundByName(String),
    MultipleMembersByName(String),
    InvalidUserID(u64),
}

impl error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            // CommandError::WrongArgCount { expected, provided } => {
            //     if expected > provided {
            //         write!(
            //             f,
            //             "Too many arguments were provided! Expected {}, but found {}",
            //             expected, provided
            //         )
            //     } else {
            //         write!(
            //             f,
            //             "Not enough arguments were provided! Expected {}, but found {}",
            //             expected, provided
            //         )
            //     }
            // }
            CommandError::NoDM => write!(f, "You can not use this command in DMs"),
        }
    }
}

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MemberNotFoundById(id) =>write!(f, "no member with userid ``{}`` found on this server", id),
            ParseError::MissingArgument => write!(f, "You are missing a requried argument"),
            ParseError::MemberNotFoundByName(name) => write!(f, "There is nobody named ``{}`` on this server", name),
            ParseError::MultipleMembersByName(name) => write!(f, "Multiple members who's name starts with ``{}`` found, please use their full name and discriminator", name),
            ParseError::InvalidUserID(id) => write!(f, "``{}`` is not a valid discord userid", id)
        }
    }
}

#[derive(Debug)]
pub enum FetchError {
    ShouldExist,
}

impl error::Error for FetchError {}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FetchError::ShouldExist => write!(f, "The provided ID doesn't exist!"),
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CmdError(e) => write!(f, "{}", e),
            Error::InvalidSession => {
                write!(f, "The gateway invalidated our session unrecoverably!")
            }
            // For errors that actually happen during runtime, we can have the logging macros here too
            Error::MissingToken => write!(f, "The bot was missing its token, unable to start!"),
            Error::NoConfig => write!(f, "The config file couldn't be found, unable to start!"),
            Error::InvalidConfig => write!(f, "The config file was not in the correct format!"),
            Error::InvalidLoggingWebhook(wurl) => write!(f, "The webhook URL {} was invalid", wurl),
            Error::NoLoggingSpec => write!(f, "The logging configuration couldn't be found!"),
            Error::IoError(e) => write!(f, "An IO error occurred during a task: {}", e),
            Error::TwilightHttp(e) => {
                write!(f, "An error occurred making a Discord request: {}", e)
            }
            Error::TwilightCluster(e) => write!(f, "An error occurred on a cluster request: {}", e),
            Error::Cache(e) => write!(
                f,
                "An error occured attempting to fetch an object from the cache: {}",
                e
            ),
            Error::Database(e) => write!(f, "A database error occurred: {}", e),
            Error::DatabaseAction(e) => write!(f, "{}", e),
            Error::DatabaseMigration(e) => write!(f, "Failed to migrate the database: {}", e),
            Error::Pool(e) => write!(f, "An error occurred in the database pool: {}", e),
            Error::UnknownEmoji(e) => write!(f, "Unknown emoji: {}", e),
            Error::Serde(e) => write!(f, "Serde error: {}", e),
            Error::ParseError(e) => write!(f, "{}", e),
        }
    }
}

impl From<CommandError> for Error {
    fn from(e: CommandError) -> Self {
        Error::CmdError(e)
    }
}

impl From<FetchError> for Error {
    fn from(e: FetchError) -> Self {
        Error::DatabaseAction(e)
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

impl From<twilight_cache_inmemory::InMemoryCacheError> for Error {
    fn from(e: twilight_cache_inmemory::InMemoryCacheError) -> Self {
        Error::Cache(e)
    }
}

impl From<tokio_postgres::error::Error> for Error {
    fn from(e: tokio_postgres::error::Error) -> Self {
        Error::Database(e)
    }
}

impl From<PoolError> for Error {
    fn from(e: PoolError) -> Self {
        Error::Pool(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Error::Serde(e)
    }
}
