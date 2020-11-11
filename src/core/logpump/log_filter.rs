use super::DataLessLogType;
use serde::{Deserialize, Serialize};
use twilight_model::id::{ChannelId, UserId};

#[derive(Deserialize, Serialize, Debug)]
pub struct LogFilter {
    log_types: Vec<DataLessLogType>,
    source_channels: Vec<ChannelId>,
    source_users: Vec<UserId>,
}

impl LogFilter {
    pub fn matches(
        &self,
        log_type: &DataLessLogType,
        source_channel: &Option<ChannelId>,
        source_user: &UserId,
    ) -> bool {
        if self.log_types.contains(log_type) {
            return true;
        }
        if let Some(channel) = source_channel {
            if self.source_channels.contains(channel) {
                return true;
            }
        }

        if self.source_users.contains(source_user) {
            return true;
        }
        false
    }
}
