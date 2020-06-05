use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandNode::{CommandNodeInner, GroupNode};
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::utils::Error;

pub type CommandResult = Result<(), Error>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
pub type CommandHandler =
    Box<dyn Fn(CommandContext, Message, Parser) -> CommandResultOuter + Send + Sync>;

pub struct Command {
    name: String,
    handler: CommandHandler,
}

impl Command {
    pub fn new(name: String, handler: CommandHandler) -> Self {
        Command { name, handler }
    }
}

pub enum CommandNode {
    CommandNodeInner {
        command: Command,
    },
    GroupNode {
        name: String,
        handler: Option<CommandHandler>,
        sub_nodes: HashMap<String, CommandNode>,
    },
}

impl CommandNode {
    pub fn create_command(name: String, handler: CommandHandler) -> Self {
        CommandNodeInner {
            command: Command { name, handler },
        }
    }

    pub fn create_node(
        name: String,
        handler: Option<CommandHandler>,
        sub_nodes: HashMap<String, CommandNode>,
    ) -> Self {
        GroupNode {
            name,
            handler,
            sub_nodes,
        }
    }

    pub fn get_name(&self) -> &str {
        match &self {
            CommandNode::CommandNodeInner { command } => &command.name,
            CommandNode::GroupNode { name, .. } => &name,
        }
    }

    pub fn get(&self, target: &str) -> Option<&CommandNode> {
        match &self {
            CommandNode::CommandNodeInner { command } => None,
            CommandNode::GroupNode { sub_nodes, .. } => sub_nodes.get(target),
        }
    }

    pub async fn execute<'a>(
        &self,
        ctx: CommandContext,
        msg: Message,
        parser: Parser,
    ) -> CommandResult {
        match &self {
            CommandNode::CommandNodeInner { command } => {
                let command = &command.handler;
                command(ctx, msg, parser).await?;
                Ok(())
            }
            CommandNode::GroupNode {
                name,
                handler,
                sub_nodes,
            } => match handler {
                Some(handler) => {
                    let command = handler;
                    command(ctx, msg, parser).await?;
                    Ok(())
                }
                None => Ok(()),
            },
        }
    }
}

impl fmt::Display for CommandNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandNode::CommandNodeInner { command } => write!(f, "{}", command.name),
            CommandNode::GroupNode {
                name,
                handler,
                sub_nodes,
            } => write!(f, "{}", name),
        }
    }
}

pub enum PermMode {
    ALLOWED,
    MAYBE,
    DENIED,
}
