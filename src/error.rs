use std::{error, fmt, io};

use serde::export::Formatter;
use twilight_embed_builder::{
    EmbedAuthorNameError, EmbedBuildError, EmbedColorError, EmbedDescriptionError, EmbedFieldError, ImageSourceUrlError,
};
use twilight_gateway::cluster::{ClusterCommandError, ClusterStartError};
use twilight_gateway::{cluster, shard};
use twilight_http::request::channel::message::create_message::CreateMessageError;
use twilight_http::request::channel::message::update_message::UpdateMessageError;
use twilight_model::id::{ChannelId, GuildId, UserId};

pub type CommandResult = Result<(), CommandError>;

#[derive(Debug)]
pub enum StartupError {
    NoConfig,
    InvalidConfig,
    NoLoggingSpec,
    Twilight(twilight_http::Error),
    Sqlx(sqlx::Error),
    DarkRedis(darkredis::Error),
    ClusterStart(ClusterStartError),
    Io(io::Error),
}

#[derive(Debug)]
pub enum ColdResumeError {
    MissingData(String),
    Database(DatabaseError),
}

impl error::Error for ColdResumeError {}

impl fmt::Display for ColdResumeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ColdResumeError::MissingData(e) => write!(f, "Cold resume data missing: {}", e),
            ColdResumeError::Database(e) => write!(f, "Database failure: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum ApiCommunicaionError {
    Deseralizing(serde_json::Error),
    Serializing(serde_json::Error),
    Redis(darkredis::Error),
}

impl error::Error for ApiCommunicaionError {}

impl fmt::Display for ApiCommunicaionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ApiCommunicaionError::Deseralizing(e) => write!(f, "Failed to deserialize api message: {}", e),
            ApiCommunicaionError::Serializing(e) => write!(f, "Failed to serialize message for the api: {}", e),
            ApiCommunicaionError::Redis(e) => write!(f, "Redis failure: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum EventHandlerError {
    InvalidSession(u64),
    Gateway(shard::CommandError),
    TwilightCluster(cluster::ClusterCommandError),
    UnknownGuild(GuildId),
    UnknownChannel(ChannelId),
    UnknownUser(UserId),
    Reactor(ReactorError),
    Database(DatabaseError),
    Twilight(twilight_http::Error),
}
impl error::Error for EventHandlerError {}

impl fmt::Display for EventHandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EventHandlerError::InvalidSession(e) => write!(f, "Our gateway session died: {}", e),
            EventHandlerError::Gateway(e) => write!(f, "Gateway error: {}", e),
            EventHandlerError::TwilightCluster(e) => write!(f, "Gateway command error: {}", e),
            EventHandlerError::UnknownGuild(e) => write!(f, "Event recieved for unknown guild {}", e),
            EventHandlerError::UnknownChannel(e) => write!(f, "Event received for unknown channel {}", e),
            EventHandlerError::UnknownUser(e) => write!(f, "Event received for unknown user {}", e),
            EventHandlerError::Reactor(e) => write!(f, "Message reactor failure: {}", e),
            EventHandlerError::Database(e) => write!(f, "Database interaction failed: {}", e),
            EventHandlerError::Twilight(e) => write!(f, "Failed to interact with the discord api: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum ReactorError {
    Database(DatabaseError),
    TwilightHttp(twilight_http::Error),
    Message(MessageError),
}

impl error::Error for ReactorError {}

impl fmt::Display for ReactorError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ReactorError::Database(e) => write!(f, "Database failure: {}", e),
            ReactorError::TwilightHttp(e) => write!(f, "Failed to interact with the discord api: {}", e),
            ReactorError::Message(e) => write!(f, "Message operation failed: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum MessageError {
    Create(CreateMessageError),
    Update(UpdateMessageError),
    EmbedBuild(EmbedBuildError),
    EmbedField(EmbedFieldError),
    EmbedDescription(EmbedDescriptionError),
    EmbedColor(EmbedColorError),
    EmbedAuthorName(EmbedAuthorNameError),
    ImageSourceUrl(ImageSourceUrlError),
}

impl error::Error for MessageError {}

impl fmt::Display for MessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            MessageError::Create(e) => write!(f, "Failed to create message: {}", e),
            MessageError::Update(e) => write!(f, "Failed to create message update: {}", e),
            MessageError::EmbedBuild(e) => write!(f, "Failed to assemble embed: {}", e),
            MessageError::EmbedField(e) => write!(f, "Failed to create embed field: {}", e),
            MessageError::EmbedDescription(e) => write!(f, "Failed to set embed description: {}", e),
            MessageError::EmbedColor(e) => write!(f, "Failed to set embed color: {}", e),
            MessageError::EmbedAuthorName(e) => write!(f, "Failed to set embed author name: {}", e),
            MessageError::ImageSourceUrl(e) => write!(f, "Failed to set embed image url: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum DatabaseError {
    Sqlx(sqlx::Error),
    Deserializing(serde_json::Error),
    Serializing(serde_json::Error),
    Darkredis(darkredis::Error),
}

impl error::Error for DatabaseError {}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::Sqlx(e) => write!(f, "Database failure: {:?}", e),
            DatabaseError::Deserializing(e) => write!(f, "Failed to deserialize: {}", e),
            DatabaseError::Serializing(e) => write!(f, "Failed to seralize: {}", e),
            DatabaseError::Darkredis(e) => write!(f, "Redis failure: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum EmojiError {
    UnknownEmoji(String),
}

impl error::Error for EmojiError {}

impl fmt::Display for EmojiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EmojiError::UnknownEmoji(e) => write!(f, "Unknown emoji: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum ApiMessageError {}

impl error::Error for ApiMessageError {}

impl fmt::Display for ApiMessageError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

#[derive(Debug)]
pub enum CommandError {
    NoDM,
    InvalidPermissions,
    ParseError(ParseError),
    OtherFailure(OtherFailure),
}

impl error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CommandError::NoDM => write!(f, "You can not use this command in DMs"),
            CommandError::InvalidPermissions => write!(f, "You don't have the permissions to run this command!"),
            CommandError::ParseError(e) => write!(f, "Failed to parse the command arguments!\n``{}``", e),
            CommandError::OtherFailure(_) => write!(f, "Unexpected error while executing the command, please report this on the support server if it keeps happening"),
        }
    }
}

#[derive(Debug)]
pub enum OtherFailure {
    ShardOrCluster(String),
    TwilightHttp(twilight_http::Error),
    DatabaseError(DatabaseError),
    CorruptCache,
    Message(MessageError),
}

impl error::Error for OtherFailure {}

impl fmt::Display for OtherFailure {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OtherFailure::DatabaseError(e) => write!(f, "Database error: {}", e),
            OtherFailure::CorruptCache => write!(f, "Cache is corrupted!"),
            OtherFailure::ShardOrCluster(e) => write!(f, "Shard command failed: {}", e),
            OtherFailure::TwilightHttp(e) => write!(f, "Something when wrong interacting with the discord api: {}", e),
            OtherFailure::Message(e) => write!(f, "Failed to construct a message: {}", e),
        }
    }
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
    CorruptCache,
    NoDm,
    Other(OtherFailure),
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
            ParseError::CorruptCache => write!(f, "While processing this command cache corruption was detected, command execution was aborted and a cache reset is in progress, please try again in a few minutes"),
            ParseError::NoDm => write!(f, "This can not be used in DMs"),
            ParseError::Other(_) => write!(f, "An unexpected error occurred trying to parse and retrieve this")
        }
    }
}
impl fmt::Display for StartupError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            StartupError::NoConfig => write!(f, "Unable to locate the config file"),
            StartupError::InvalidConfig => write!(f, "Unable to load the config file"),
            StartupError::NoLoggingSpec => write!(f, "Problem with the log spec file"),
            StartupError::Twilight(e) => write!(f, "Twilight error during startup, unable to continue: {}", e),
            StartupError::Sqlx(e) => write!(f, "Unable to create database pool: {:?}", e),
            StartupError::DarkRedis(e) => write!(f, "Unable to create redis database pool: {}", e),
            StartupError::ClusterStart(e) => write!(f, "The cluster failed to start: {}", e),
            StartupError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

pub enum LogError {
    Database(DatabaseError),
    Twilight,
}

impl From<twilight_http::Error> for StartupError {
    fn from(e: twilight_http::Error) -> Self {
        StartupError::Twilight(e)
    }
}

impl From<sqlx::error::Error> for StartupError {
    fn from(e: sqlx::error::Error) -> Self {
        StartupError::Sqlx(e)
    }
}

impl From<darkredis::Error> for StartupError {
    fn from(e: darkredis::Error) -> Self {
        StartupError::DarkRedis(e)
    }
}

impl From<EmbedFieldError> for CommandError {
    fn from(e: EmbedFieldError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::EmbedField(e)))
    }
}

impl From<EmbedBuildError> for CommandError {
    fn from(e: EmbedBuildError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::EmbedBuild(e)))
    }
}

impl From<EmbedDescriptionError> for CommandError {
    fn from(e: EmbedDescriptionError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::EmbedDescription(e)))
    }
}

impl From<EmbedColorError> for CommandError {
    fn from(e: EmbedColorError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::EmbedColor(e)))
    }
}

impl From<EmbedAuthorNameError> for CommandError {
    fn from(e: EmbedAuthorNameError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::EmbedAuthorName(e)))
    }
}
impl From<ImageSourceUrlError> for CommandError {
    fn from(e: ImageSourceUrlError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::ImageSourceUrl(e)))
    }
}

impl From<ClusterStartError> for StartupError {
    fn from(e: ClusterStartError) -> Self {
        StartupError::ClusterStart(e)
    }
}

impl From<CreateMessageError> for CommandError {
    fn from(e: CreateMessageError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::Create(e)))
    }
}

impl From<twilight_http::Error> for CommandError {
    fn from(e: twilight_http::Error) -> Self {
        CommandError::OtherFailure(OtherFailure::TwilightHttp(e))
    }
}

impl From<OtherFailure> for CommandError {
    fn from(e: OtherFailure) -> Self {
        CommandError::OtherFailure(e)
    }
}
impl From<darkredis::Error> for ParseError {
    fn from(e: darkredis::Error) -> Self {
        ParseError::Other(OtherFailure::DatabaseError(DatabaseError::Darkredis(e)))
    }
}

impl From<twilight_http::Error> for ParseError {
    fn from(e: twilight_http::Error) -> Self {
        ParseError::Other(OtherFailure::TwilightHttp(e))
    }
}

impl From<darkredis::Error> for ApiCommunicaionError {
    fn from(e: darkredis::Error) -> Self {
        ApiCommunicaionError::Redis(e)
    }
}

impl From<DatabaseError> for ParseError {
    fn from(e: DatabaseError) -> Self {
        ParseError::Other(OtherFailure::DatabaseError(e))
    }
}

impl From<ClusterCommandError> for EventHandlerError {
    fn from(e: ClusterCommandError) -> Self {
        EventHandlerError::TwilightCluster(e)
    }
}

impl From<ReactorError> for EventHandlerError {
    fn from(e: ReactorError) -> Self {
        EventHandlerError::Reactor(e)
    }
}
impl From<DatabaseError> for EventHandlerError {
    fn from(e: DatabaseError) -> Self {
        EventHandlerError::Database(e)
    }
}

impl From<darkredis::Error> for DatabaseError {
    fn from(e: darkredis::Error) -> Self {
        DatabaseError::Darkredis(e)
    }
}

impl From<darkredis::Error> for ColdResumeError {
    fn from(e: darkredis::Error) -> Self {
        ColdResumeError::Database(DatabaseError::Darkredis(e))
    }
}

impl From<UpdateMessageError> for CommandError {
    fn from(e: UpdateMessageError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(MessageError::Update(e)))
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(e: sqlx::Error) -> Self {
        DatabaseError::Sqlx(e)
    }
}

impl From<DatabaseError> for ReactorError {
    fn from(e: DatabaseError) -> Self {
        ReactorError::Database(e)
    }
}
impl From<twilight_http::Error> for EventHandlerError {
    fn from(e: twilight_http::Error) -> Self {
        EventHandlerError::Twilight(e)
    }
}

impl From<twilight_http::Error> for ReactorError {
    fn from(e: twilight_http::Error) -> Self {
        ReactorError::TwilightHttp(e)
    }
}
impl From<DatabaseError> for ColdResumeError {
    fn from(e: DatabaseError) -> Self {
        ColdResumeError::Database(e)
    }
}

impl From<ParseError> for CommandError {
    fn from(e: ParseError) -> Self {
        match e {
            ParseError::NoDm => CommandError::NoDM,
            e => CommandError::ParseError(e),
        }
    }
}

impl From<DatabaseError> for CommandError {
    fn from(e: DatabaseError) -> Self {
        CommandError::OtherFailure(OtherFailure::DatabaseError(e))
    }
}

impl From<UpdateMessageError> for ReactorError {
    fn from(e: UpdateMessageError) -> Self {
        ReactorError::Message(MessageError::Update(e))
    }
}

impl From<EmbedFieldError> for ReactorError {
    fn from(e: EmbedFieldError) -> Self {
        ReactorError::Message(MessageError::EmbedField(e))
    }
}

impl From<EmbedBuildError> for ReactorError {
    fn from(e: EmbedBuildError) -> Self {
        ReactorError::Message(MessageError::EmbedBuild(e))
    }
}

impl From<EmbedDescriptionError> for ReactorError {
    fn from(e: EmbedDescriptionError) -> Self {
        ReactorError::Message(MessageError::EmbedDescription(e))
    }
}

impl From<EmbedColorError> for ReactorError {
    fn from(e: EmbedColorError) -> Self {
        ReactorError::Message(MessageError::EmbedColor(e))
    }
}

impl From<EmbedAuthorNameError> for ReactorError {
    fn from(e: EmbedAuthorNameError) -> Self {
        ReactorError::Message(MessageError::EmbedAuthorName(e))
    }
}
impl From<ImageSourceUrlError> for ReactorError {
    fn from(e: ImageSourceUrlError) -> Self {
        ReactorError::Message(MessageError::ImageSourceUrl(e))
    }
}

impl From<MessageError> for ReactorError {
    fn from(e: MessageError) -> Self {
        ReactorError::Message(e)
    }
}

impl From<MessageError> for CommandError {
    fn from(e: MessageError) -> Self {
        CommandError::OtherFailure(OtherFailure::Message(e))
    }
}

impl From<ImageSourceUrlError> for MessageError {
    fn from(e: ImageSourceUrlError) -> Self {
        MessageError::ImageSourceUrl(e)
    }
}

impl From<EmbedAuthorNameError> for MessageError {
    fn from(e: EmbedAuthorNameError) -> Self {
        MessageError::EmbedAuthorName(e)
    }
}

impl From<EmbedDescriptionError> for MessageError {
    fn from(e: EmbedDescriptionError) -> Self {
        MessageError::EmbedDescription(e)
    }
}

impl From<io::Error> for StartupError {
    fn from(e: io::Error) -> Self {
        StartupError::Io(e)
    }
}

impl From<EmbedBuildError> for MessageError {
    fn from(e: EmbedBuildError) -> Self {
        MessageError::EmbedBuild(e)
    }
}
