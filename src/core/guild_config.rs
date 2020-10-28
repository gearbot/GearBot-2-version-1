use serde::{Deserialize, Serialize};
use twilight_model::guild::Permissions;
use twilight_model::id::{ChannelId, RoleId, UserId};
use unic_langid::LanguageIdentifier;

use crate::commands::meta::nodes::GearBotPermissions;
use crate::core::{DataLessLogType, LogFilter};
use crate::translation::DEFAULT_LANG;
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug)]
pub struct GuildConfig {
    pub prefix: String,
    pub log_style: LogStyle,
    pub message_logs: MessageLogs,
    pub language: LanguageIdentifier,
    pub permission_groups: Vec<PermissionGroup>,
    pub log_channels: HashMap<ChannelId, LogChannelConfig>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PermissionGroup {
    pub priority: u8,
    pub name: String,
    pub granted_perms: GearBotPermissions,
    pub denied_perms: GearBotPermissions,
    pub discord_perms: Option<Permissions>,
    pub roles: Vec<RoleId>,
    pub needs_all: bool,
    pub users: Vec<UserId>,
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
#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum LogCategory {
    TEST,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct LogChannelConfig {
    pub categories: Vec<LogCategory>,
    pub disabled_keys: Vec<DataLessLogType>,
    pub style: LogStyle,
    pub filters: Vec<LogFilter>,
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
            language: DEFAULT_LANG,
            permission_groups: vec![
                PermissionGroup {
                    priority: 0,
                    name: String::from("All members"),
                    granted_perms: GearBotPermissions::BASIC_GROUP | GearBotPermissions::EMOJI_LIST_COMMAND,
                    denied_perms: GearBotPermissions::empty(),
                    discord_perms: Some(Permissions::empty()),
                    roles: vec![],
                    needs_all: false,
                    users: vec![],
                },
                PermissionGroup {
                    priority: 25,
                    name: String::from("Moderators"),
                    granted_perms: GearBotPermissions::BASIC_GROUP
                        | GearBotPermissions::EMOJI_LIST_COMMAND
                        | GearBotPermissions::MODERATION_GROUP
                        | GearBotPermissions::READ_CONFIG,
                    denied_perms: GearBotPermissions::empty(),
                    discord_perms: Some(Permissions::BAN_MEMBERS),
                    roles: vec![],
                    needs_all: false,
                    users: vec![],
                },
                PermissionGroup {
                    priority: 50,
                    name: String::from("Administrators"),
                    granted_perms: GearBotPermissions::BASIC_GROUP
                        | GearBotPermissions::MODERATION_GROUP
                        | GearBotPermissions::MISC_GROUP
                        | GearBotPermissions::GUILD_ADMIN_GROUP,
                    denied_perms: GearBotPermissions::empty(),
                    discord_perms: Some(Permissions::ADMINISTRATOR),
                    roles: vec![],
                    needs_all: false,
                    users: vec![],
                },
            ],
            log_channels: HashMap::new(),
        }
    }
}
