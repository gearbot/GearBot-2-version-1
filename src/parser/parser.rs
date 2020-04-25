use std::ops::Add;
use std::sync::Arc;

use log::{debug, info, trace};
use twilight::model::gateway::payload::MessageCreate;

use crate::commands;
use crate::commands::meta::nodes::CommandNode;
use crate::core::Context;
use crate::utils::errors::Error;

pub struct Parser {
    parts: Vec<String>,
    index: usize,
}

impl Parser {
    fn new (content: &String) -> Self {
        Parser {
            parts: content.split_whitespace().map(std::borrow::ToOwned::to_owned).collect::<Vec<String>>(),
            index: 0
        }
    }

    pub fn get_command(&mut self) -> Option<&CommandNode> {
        let mut done = false;
        let mut nodes: Vec<&CommandNode> = vec![];
        let mut to_search: &CommandNode = commands::get_root();
        let mut to_execute: Option<&CommandNode> = None;
        while self.index < self.parts.len() && !done {
            let target = &self.parts[self.index];

            let node = to_search.get(target);
            match node {
                Some(node) => {
                    to_search = node;
                    debug!("Found a command node: {}", node.get_name());
                    self.index += 1;
                    nodes.push(node);
                    to_execute = Some(node);
                }
                None => {
                    debug!("No more command nodes found");
                    done = true;
                }
            }
        };

        to_execute
        // None
    }

    pub async fn figure_it_out(message: Box<MessageCreate>, ctx: Arc<Context>) -> Result<(), Error> {
        //TODO: verify permissions
        let test = message.0.clone();
        let mut parser = Parser::new(&message.0.content);
        debug!("Parser processing message: {:?}", &message.content);

        //TODO: walk the stack to validate permissions
        let command = parser.get_command();

        match command
        {
            Some(node) => {
                node.execute(ctx, test, Parser::new(&message.0.content)).await?;
                Ok(())
            },
            None => Ok(())
        }
    }
}
