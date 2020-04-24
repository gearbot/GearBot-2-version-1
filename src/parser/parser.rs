use std::ops::Add;
use std::sync::Arc;

use log::{debug, info, trace};
use twilight::model::gateway::payload::MessageCreate;

use crate::core::Context;
use crate::gears;
use crate::utils::errors::Error;

pub struct Parser<'a> {
    parts: Vec<&'a str>,
    index: usize,
}

impl<'a> Parser<'a> {
    pub async fn figure_it_out(message: Box<MessageCreate>, ctx: Arc<Context>) -> Result<(), Error> {
        //TODO: verify permissions
        let parts: Vec<&str> = message.content.split_whitespace().collect();
        debug!("Parser processing message: {:?}", message.content);
        let mut nodes = vec![];
        let mut index = 0;
        let mut to_execute = None;
        let mut done = false;
        while index < parts.len() && !done {
            let target = parts[index];

            let node = gears::COMMANDS.get(target);
            match node {
                Some(node) => {
                    debug!("Found a command node: {}", node.get_name());
                    index += 1;
                    nodes.push(node);
                    to_execute = Some(node);
                }
                None => {
                    debug!("No more command nodes found");
                    done = true;
                }
            }
        }


        //TODO: walk the stack to validate permissions
        
        
        let parser = Parser {
            parts,
            index
        };

        match to_execute
        {
            Some(node) => {
                node.execute(ctx, message.0).await?;
                Ok(())
            },
            None => Ok(())
        }
    }
}
