use crate::core::logpump::log_type::LogType;
use twilight_model::id::{ChannelId, GuildId, UserId};

#[derive(Debug)]
pub struct LogData {
    pub log_type: LogType,
    pub guild: GuildId,
    pub source_channel: Option<ChannelId>,
    pub source_user: UserId,
}
