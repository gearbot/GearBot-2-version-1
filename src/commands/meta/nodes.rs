use std::collections::HashMap;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandNode::{CommandNodeInner, GroupNode};
use crate::core::Context;
use crate::parser::Parser;
use crate::utils::Error;

pub type CommandResult = Result<(), Error>;
pub type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
pub type CommandHandler =
    Box<dyn Fn(Arc<Context>, Message, Parser) -> CommandResultOuter + Send + Sync>;

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
            CommandNode::GroupNode {
                name,
                handler,
                sub_nodes,
            } => &name,
        }
    }

    pub fn get(&self, target: &str) -> Option<&CommandNode> {
        match &self {
            CommandNode::CommandNodeInner { command } => None,
            CommandNode::GroupNode {
                name,
                handler,
                sub_nodes,
            } => sub_nodes.get(target),
        }
    }

    pub async fn execute(&self, ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
        match &self {
            CommandNode::CommandNodeInner { command } => {
                let test = &command.handler;
                test(ctx, msg, parser).await?;
                Ok(())
            }
            CommandNode::GroupNode {
                name,
                handler,
                sub_nodes,
            } => match handler {
                Some(handler) => {
                    let test = handler;
                    test(ctx, msg, parser).await?;
                    Ok(())
                }
                None => Ok(()),
            },
        }
    }
}

impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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
