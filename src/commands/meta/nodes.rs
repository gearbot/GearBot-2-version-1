use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use twilight::model::guild::Permissions;

use crate::core::CommandContext;
use crate::translation::GearBotString;
use crate::utils::Error;

pub type CommandResult = Result<(), Error>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
pub type CommandHandler = Box<dyn Fn(CommandContext) -> CommandResultOuter + Send + Sync>;

pub struct RootNode {
    pub all_commands: HashMap<String, Arc<CommandNode>>,
    pub command_list: Vec<Arc<CommandNode>>,
    pub by_group: HashMap<CommandGroup, Vec<Arc<CommandNode>>>,
    pub groups: Vec<CommandGroup>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum CommandGroup {
    Basic,
    GuildAdmin,
    Moderation,
    BotAdmin,
    Misc,
}

impl CommandGroup {
    pub fn get_permission(&self) -> GearBotPermissions {
        match self {
            CommandGroup::Basic => GearBotPermissions::BASIC_GROUP,
            CommandGroup::GuildAdmin => GearBotPermissions::GUILD_ADMIN_GROUP,
            CommandGroup::Moderation => GearBotPermissions::MODERATION_GROUP,
            CommandGroup::BotAdmin => GearBotPermissions::BOT_ADMIN,
            CommandGroup::Misc => GearBotPermissions::MISC_GROUP,
        }
    }

    pub fn get_name(&self) -> &'static str {
        match self {
            CommandGroup::Basic => "basic",
            CommandGroup::GuildAdmin => "guild_admin",
            CommandGroup::Moderation => "moderation",
            CommandGroup::BotAdmin => "bot_admin",
            CommandGroup::Misc => "misc",
        }
    }
}

bitflags! {
    pub struct GearBotPermissions: u64 {
        const BOT_ADMIN             = 0x000_001;
        const BASIC_GROUP           = 0x000_002;
        const ABOUT_COMMAND         = 0x000_004;
        const COINFLIP_COMMAND      = 0x000_008;
        const PING_COMMAND          = 0x000_010;
        const QUOTE_COMMAND         = 0x000_020;
        const UID_COMMAND           = 0x000_040;
        const GUILD_ADMIN_GROUP     = 0x000_080;
        const CONFIG_COMMAND        = 0x000_100;
        const READ_CONFIG           = 0x000_200;
        const WRITE_CONFIG          = 0x000_400;
        const MODERATION_GROUP      = 0x000_800;
        const USERINFO_COMMAND      = 0x001_000;
        const HELP_COMMAND          = 0x002_000;
        const MISC_GROUP            = 0x004_000;
        const EMOJI_COMMAND         = 0x008_000;
        const EMOJI_LIST_COMMAND    = 0x010_000;
    }
}

impl<'de> Deserialize<'de> for GearBotPermissions {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::from_bits_truncate(u64::deserialize(deserializer)?))
    }
}

impl Serialize for GearBotPermissions {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.bits())
    }
}

pub struct CommandNode {
    pub name: String,
    pub handler: Option<CommandHandler>,
    pub sub_nodes: HashMap<String, Arc<CommandNode>>,
    pub node_list: Vec<Arc<CommandNode>>,
    pub bot_permissions: Permissions,
    pub command_permission: GearBotPermissions,
    pub group: CommandGroup,
    pub aliases: Vec<String>,
}
