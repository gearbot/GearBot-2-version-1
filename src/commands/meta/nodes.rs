use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils::Error;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use twilight::model::guild::Permissions;

pub type CommandResult = Result<(), Error>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
pub type CommandHandler = Box<dyn Fn(CommandContext, Parser) -> CommandResultOuter + Send + Sync>;

pub struct RootNode {
    pub all_commands: HashMap<String, CommandNode>,
}

impl RootNode {
    pub fn by_group(&self) -> HashMap<CommandGroup, Vec<&CommandNode>> {
        let mut out = HashMap::new();
        for node in self.all_commands.values() {
            let mut vec = match out.remove(&node.group) {
                Some(vec) => vec,
                None => vec![],
            };
            vec.push(node);
            out.insert(node.group.clone(), vec);
        }
        out
    }
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
    pub sub_nodes: HashMap<String, CommandNode>,
    pub bot_permissions: Permissions,
    pub command_permission: GearBotPermission,
    pub group: CommandGroup,
}

pub enum PermMode {
    ALLOWED,
    MAYBE,
    DENIED,
}
