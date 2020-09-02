use std::{error, fmt, io};

use serde::export::Formatter;
use twilight::gateway::cluster::ClusterStartError;
use twilight::gateway::{cluster, shard};
use twilight::http;
use twilight::http::request::channel::message::create_message::CreateMessageError;
use twilight::http::request::channel::message::update_message::UpdateMessageError;
use twilight::model::id::GuildId;
use twilight_embed_builder::{
    EmbedAuthorNameError, EmbedBuildError, EmbedColorError, EmbedDescriptionError, EmbedFieldError, ImageSourceUrlError,
};

#[derive(Debug)]
pub enum Error {
    MissingToken,
    NoConfig,
    InvalidConfig,
    InvalidLoggingWebhook(String),
    NoLoggingSpec,
    IoError(io::Error),

    TwilightHttp(http::Error),
    TwilightCluster(cluster::ClusterCommandError),
    GatewayError(shard::CommandError),
    ShardOrClusterError(String),
    InvalidSession(u64),

    Database(sqlx::error::Error),
    RedisError(darkredis::Error),

    Serde(serde_json::error::Error),
    ParseError(ParseError),
    CmdError(CommandError),
    LogError(GuildId),
    CreateMessageError(CreateMessageError),
    UpdateMessageError(UpdateMessageError),
    CacheError(String),

    PrometheusError(prometheus::Error),
    UnknownEmoji(String),
    EmbedError(String),
}

#[derive(Debug)]
pub enum CommandError {
    // WrongArgCount { expected: u8, provided: u8 },
    NoDM,
    InvalidPermissions,
}

#[derive(Debug)]
pub enum ParseError {
    MissingArgument,
    MemberNotFoundById(u64),
    MemberNotFoundByName(String),
    MultipleMembersByName(String),
    WrongArgumentType(String),
    InvalidUserID(u64),
    UnknownChannel(u64),
    NoChannelAccessBot(String),
    NoChannelAccessUser(String),
    UnknownMessage,
    NSFW,
}

impl error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::NoDM => write!(f, "You can not use this command in DMs"),
            CommandError::InvalidPermissions => write!(f, "You don't have the permissions to run this command!"),
        }
    }
}

impl error::Error for ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::MemberNotFoundById(id) => write!(f, "no member with userid ``{}`` found on this server", id),
            ParseError::MissingArgument => write!(f, "You are missing one or more required arguments"),
            ParseError::MemberNotFoundByName(name) => write!(f, "There is nobody named ``{}`` on this server", name),
            ParseError::MultipleMembersByName(name) => write!(
                f,
                "Multiple members who's name starts with ``{}`` found, please use their full name and discriminator",
                name
            ),
            ParseError::WrongArgumentType(expected) => write!(
                f,
                "The wrong type was provided! Expected a {}, but got something else!",
                expected
            ),
            ParseError::InvalidUserID(id) => write!(f, "``{}`` is not a valid discord userid", id),
            ParseError::UnknownChannel(id) => write!(f, "Unable to find any channel with id ``{}``", id),
            ParseError::NoChannelAccessBot(_) => write!(f, "I do not have access to that channel!"),
            ParseError::NoChannelAccessUser(_) => write!(f, "You do not have access to that channel!"),
            ParseError::UnknownMessage => write!(f, "Unable to find that message"),
            ParseError::NSFW => write!(
                f,
                "That message originates in a nsfw channel while this is not a nsfw channel, unable to comply"
            ),
        }
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::CmdError(e) => write!(f, "{}", e),
            Error::InvalidSession(shard) => write!(
                f,
                "The gateway invalidated our session unrecoverably for shard {}!",
                shard
            ),
            // For errors that actually happen during runtime, we can have the logging macros here too
            Error::MissingToken => write!(f, "The bot was missing its token, unable to start!"),
            Error::NoConfig => write!(f, "The config file couldn't be found, unable to start!"),
            Error::InvalidConfig => write!(f, "The config file was not in the correct format!"),
            Error::InvalidLoggingWebhook(wurl) => write!(f, "The webhook URL {} was invalid", wurl),
            Error::NoLoggingSpec => write!(f, "The logging configuration couldn't be found!"),
            Error::IoError(e) => write!(f, "An IO error occurred during a task: {}", e),
            Error::TwilightHttp(e) => write!(f, "An error occurred making a Discord request: {}", e),
            Error::TwilightCluster(e) => write!(f, "An error occurred on a cluster request: {}", e),
            Error::Database(e) => write!(f, "A database error occurred: {}", e),
            Error::UnknownEmoji(e) => write!(f, "Unknown emoji: {}", e),
            Error::Serde(e) => write!(f, "Serde error: {}", e),
            Error::ParseError(e) => write!(f, "{}", e),
            Error::LogError(guild_id) => write!(
                f,
                "Something went horribly wrong when trying to push to the logpump for guild {}",
                guild_id
            ),
            Error::CreateMessageError(e) => write!(f, "Error creating message: {}", e),
            Error::UpdateMessageError(e) => write!(f, "Error updating message: {}", e),
            Error::CacheError(e) => write!(f, "An error occured with a cache task: {}", e),
            Error::RedisError(e) => write!(f, "Error communicating with the redis cache: {}", e),
            Error::PrometheusError(e) => write!(f, "Prometheus stat tracking failed: {}", e),
            Error::GatewayError(e) => write!(f, "Gateway error: {}", e),
            Error::ShardOrClusterError(e) => write!(f, "A cluster or shard error occured: {}", e),
            Error::EmbedError(e) => write!(f, "An embed was constructed invalidly: {}", e),
        }
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::ParseError(e)
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

impl From<cluster::ClusterCommandError> for Error {
    fn from(e: cluster::ClusterCommandError) -> Self {
        Error::TwilightCluster(e)
    }
}

impl From<sqlx::error::Error> for Error {
    fn from(e: sqlx::error::Error) -> Self {
        Error::Database(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Error::Serde(e)
    }
}

impl From<CreateMessageError> for Error {
    fn from(e: CreateMessageError) -> Self {
        Error::CreateMessageError(e)
    }
}

impl From<UpdateMessageError> for Error {
    fn from(e: UpdateMessageError) -> Self {
        Error::UpdateMessageError(e)
    }
}
impl From<darkredis::Error> for Error {
    fn from(e: darkredis::Error) -> Self {
        Error::RedisError(e)
    }
}

impl From<shard::CommandError> for Error {
    fn from(e: shard::CommandError) -> Self {
        Error::GatewayError(e)
    }
}

impl From<ClusterStartError> for Error {
    fn from(e: ClusterStartError) -> Self {
        Error::ShardOrClusterError(e.to_string())
    }
}

impl From<EmbedFieldError> for Error {
    fn from(e: EmbedFieldError) -> Self {
        Error::EmbedError(e.to_string())
    }
}

impl From<EmbedBuildError> for Error {
    fn from(e: EmbedBuildError) -> Self {
        Error::EmbedError(e.to_string())
    }
}

impl From<EmbedDescriptionError> for Error {
    fn from(e: EmbedDescriptionError) -> Self {
        Error::EmbedError(e.to_string())
    }
}

impl From<EmbedColorError> for Error {
    fn from(e: EmbedColorError) -> Self {
        Error::EmbedError(e.to_string())
    }
}

impl From<EmbedAuthorNameError> for Error {
    fn from(e: EmbedAuthorNameError) -> Self {
        Error::EmbedError(e.to_string())
    }
}
impl From<ImageSourceUrlError> for Error {
    fn from(e: ImageSourceUrlError) -> Self {
        Error::EmbedError(e.to_string())
    }
}
