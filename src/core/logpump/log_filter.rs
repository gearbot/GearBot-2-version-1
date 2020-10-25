use crate::core::{DataLessLogType, LogData, LogType};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
        source_user: &Option<UserId>,
    ) -> bool {
        if self.log_types.contains(log_type) {
            return true;
        }
        if let Some(channel) = source_channel {
            if self.source_channels.contains(channel) {
                return true;
            }
        }

        if let Some(user) = source_user {
            if self.source_users.contains(user) {
                return true;
            }
        }
        false
    }
}
