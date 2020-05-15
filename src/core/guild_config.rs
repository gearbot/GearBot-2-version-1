use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GuildConfig {
    pub prefix: String,
    pub log_style: LogStyle,
    pub message_logs: MessageLogs,
}

unsafe impl Send for GuildConfig {}

#[derive(Deserialize, Serialize, Debug)]
pub struct MessageLogs {
    pub enabled: bool,
    pub ignored_users: Vec<u64>,
    pub ignored_channels: Vec<u64>,
    pub ignore_bots: bool,
}
unsafe impl Send for MessageLogs {}

#[derive(Deserialize, Serialize, Debug)]
pub enum LogStyle {
    Text,
    Embed,
}
unsafe impl Send for LogStyle {}

#[derive(Deserialize, Serialize, Debug)]
pub struct LogChannelConfig {}

unsafe impl Send for LogChannelConfig {}

#[derive(Deserialize, Serialize, Debug)]
pub enum LogCategories {}

unsafe impl Send for LogCategories {}

#[derive(Deserialize, Serialize, Debug)]
pub enum LogSubCategory {}

unsafe impl Send for LogSubCategory {}

impl Default for GuildConfig {
    fn default() -> Self {
        GuildConfig {
            prefix: "!".to_string(),
            log_style: LogStyle::Text,
            message_logs: MessageLogs {
                enabled: false,
                ignored_users: vec![],
                ignored_channels: vec![],
                ignore_bots: true,
            },
        }
    }
}
