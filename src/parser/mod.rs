use std::sync::Arc;

use log::debug;
use twilight::model::gateway::payload::MessageCreate;

use crate::commands;
use crate::commands::meta::nodes::CommandNode;
use crate::core::Context;
use crate::utils::Error;

#[derive(Debug, Clone)]
pub struct Parser {
    pub parts: Vec<String>,
    index: usize,
}

impl Parser {
    fn new(content: &String) -> Self {
        Parser {
            parts: content
                .split_whitespace()
                .map(std::borrow::ToOwned::to_owned)
                .collect::<Vec<String>>(),
            index: 0,
        }
    }

    pub fn get_command(&mut self) -> Vec<&CommandNode> {
        let mut done = false;
        let mut nodes: Vec<&CommandNode> = vec![];
        let mut to_search: &CommandNode = commands::get_root();
        while self.index < self.parts.len() && !done {
            let target = &self.parts[self.index];

            let node = to_search.get(target);
            match node {
                Some(node) => {
                    to_search = node;
                    debug!("Found a command node: {}", node.get_name());
                    self.index += 1;
                    nodes.push(node);
                }
                None => {
                    debug!("No more command nodes found");
                    done = true;
                }
            }
        }

        nodes
        // None
    }

    pub async fn figure_it_out(
        message: Box<MessageCreate>,
        ctx: Arc<Context>,
    ) -> Result<(), Error> {
        //TODO: verify permissions
        let mut parser = Parser::new(&message.0.content);
        debug!("Parser processing message: {:?}", &message.content);

        //TODO: walk the stack to validate permissions
        let mut p = parser.clone();
        let command_nodes = p.get_command();

        match command_nodes.last() {
            Some(node) => {
                let mut name = String::from("");
                for i in 0..command_nodes.len() {
                    if i > 0 {
                        name += "__"
                    }
                    name += command_nodes[i].get_name()
                }
                debug!("Executing command: {}", name);

                node.execute(ctx, message.0, parser).await?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}
