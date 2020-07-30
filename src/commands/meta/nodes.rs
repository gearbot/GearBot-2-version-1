use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::core::CommandContext;
use crate::utils::Error;
use std::sync::Arc;
use twilight::model::guild::Permissions;

pub type CommandResult = Result<(), Error>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
pub type CommandHandler = Box<dyn Fn(CommandContext) -> CommandResultOuter + Send + Sync>;

pub struct RootNode {
    pub all_commands: HashMap<String, Arc<CommandNode>>,
    pub command_list: Vec<Arc<CommandNode>>,
    pub by_group: HashMap<CommandGroup, Vec<Arc<CommandNode>>>,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum CommandGroup {
    Basic,
    Admin,
    Moderation,
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub enum GearBotPermission {
    BasicGroup,
    AboutCommand,
    CoinflipCommand,
    PingCommand,
    QuoteCommand,
    UidCommand,
    AdminGroup,
    ConfigCommand,
    GetConfigCommand,
    SetConfigCommand,
    ModerationGroup,
    UserInfoCommand,
}

pub struct CommandNode {
    pub name: String,
    pub handler: Option<CommandHandler>,
    pub sub_nodes: HashMap<String, Arc<CommandNode>>,
    pub node_list: Vec<Arc<CommandNode>>,
    pub bot_permissions: Permissions,
    pub command_permission: GearBotPermission,
    pub group: CommandGroup,
    pub aliases: Vec<String>,
}

pub enum PermMode {
    ALLOWED,
    MAYBE,
    DENIED,
}
