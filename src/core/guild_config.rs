use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GuildConfig {
    pub prefix: String,
    pub log_style: LogStyle,
    pub message_logs: MessageLogs,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MessageLogs {
    pub enabled: bool,
    pub ignored_users: Vec<u64>,
    pub ignored_channels: Vec<u64>,
    pub ignore_bots: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub enum LogStyle {
    Text,
    Embed,
}

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
