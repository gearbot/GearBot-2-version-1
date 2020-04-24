use core::fmt;
use std::collections::HashMap;
use std::fmt::{Display, write};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use once_cell::sync::OnceCell;
use serde::export::Formatter;
use twilight::model::channel::Message;

use lazy_static::lazy_static;

use crate::core::Context;
use crate::gears::CommandNode::{CommandNodeInner, GroupNode};
use crate::parser::parser::Parser;
use crate::utils::errors::Error;

pub mod basic;
type CommandResult = Result<(), Error>;
type CommandResultOuter = Pin<Box<dyn Future<Output = CommandResult> + Send>>;
type CommandHandler = Box<dyn Fn(Arc<Context>, &Message) -> CommandResultOuter + Send + Sync>;

pub struct Command {
    name: String,
    handler: CommandHandler
}

impl Command {
    pub fn new(name: String, handler: CommandHandler) -> Self {
        Command {
            name,
            handler
        }
    }
}

pub enum CommandNode {
    CommandNodeInner{
        command: Command
    },
    GroupNode {
        name: String,
        nodes: Vec<CommandNode>,
    },
}

impl CommandNode {
    pub fn create_command(name: String, handler: CommandHandler) -> Self {
        CommandNodeInner {
            command: Command {
                name,
                handler
            }
        }
    }

    pub fn create_node(name: String, nodes: Vec<CommandNode>)->Self {
        GroupNode {
            name,
            nodes
        }
    }

    pub fn get_name(&self) -> &str {
        match &self {
            CommandNode::CommandNodeInner {command} => &command.name,
            CommandNode::GroupNode {name, nodes}  => &name
        }

    }

    pub async fn execute(&self, ctx: Arc<Context>, msg: &Message) -> CommandResult {
        match &self {
            CommandNode::CommandNodeInner {command} => {
                let test = &command.handler;
                test(ctx, msg).await?;
                Ok(())
            },
            CommandNode::GroupNode {name, nodes}  => Ok(())
        }
    }
}



impl Display for CommandNode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            CommandNode::CommandNodeInner { command } => { write!(f, "{}", command.name) },
            CommandNode::GroupNode { name, nodes } => { write!(f, "{}", name) },
        }
    }
}


pub enum PermMode {
    ALLOWED,
    MAYBE,
    DENIED,
}

lazy_static! {
 pub static ref COMMANDS: Arc<HashMap<String, CommandNode>> = {
 let mut commands : HashMap<String, CommandNode> = HashMap::new();

 //add commands here
 commands.insert(String::from("ping"), CommandNode::create_command(String::from("ping"), Box::new(|ctx, msg | Box::pin(basic::ping(ctx.clone(), msg.clone())))));



 Arc::new(commands)
 };
}