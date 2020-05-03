use std::sync::Arc;

use log::debug;
use twilight::model::gateway::payload::MessageCreate;

use crate::commands;
use crate::commands::meta::nodes::CommandNode;
use crate::core::Context;
use crate::utils::{matchers, CommandError, Error, ParseError};
use twilight::cache::twilight_cache_inmemory::model::CachedMember;
use twilight::model::gateway::presence::Presence;
use twilight::model::guild::Member;
use twilight::model::id::{GuildId, UserId};
use twilight::model::user::User;

#[derive(Clone)]
pub struct Parser {
    pub parts: Vec<String>,
    pub index: usize,
    ctx: Arc<Context>,
    shard_id: u64,
    guild_id: Option<GuildId>,
}

impl Parser {
    fn new(content: &str, ctx: Arc<Context>, shard_id: u64, guild_id: Option<GuildId>) -> Self {
        Parser {
            parts: content
                .split_whitespace()
                .map(String::from)
                .collect::<Vec<String>>(),
            index: 0,
            ctx,
            shard_id,
            guild_id,
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
                    debug!("{}", self.index);
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
        prefix: &str,
        message: Box<MessageCreate>,
        ctx: Arc<Context>,
        shard_id: u64,
    ) -> Result<(), Error> {
        //TODO: verify permissions
        let mut parser = Parser::new(
            &message.0.content[prefix.len()..],
            ctx.clone(),
            shard_id,
            message.guild_id,
        );
        debug!("Parser processing message: {:?}", &message.content);

        let mut p = parser.clone();

        let command_nodes = parser.get_command();
        match command_nodes.last() {
            Some(node) => {
                let mut name = String::new();
                for (i, node) in command_nodes.iter().enumerate() {
                    if i > 0 {
                        name += "__"
                    }
                    name += node.get_name()
                }
                debug!("Executing command: {}", name);

                p.index += command_nodes.len();
                node.execute(ctx.clone(), message.0, p).await?;
                ctx.stats.command_used(false).await;

                Ok(())
            }
            None => Ok(()),
        }
    }

    pub fn get_next(&mut self) -> Result<&str, Error> {
        if self.index == self.parts.len() {
            Err(Error::ParseError(ParseError::MissingArgument))
        } else {
            let result = &self.parts[self.index];
            self.index += 1;
            debug!("{}", self.index);
            Ok(result)
        }
    }

    pub fn has_next(&self) -> bool {
        self.index < self.parts.len()
    }

    /// parses what comes next as discord user
    pub async fn get_user(&mut self) -> Result<Arc<User>, Error> {
        let input = self.get_next()?;
        let mention = matchers::get_mention(input);
        match mention {
            // we got a mention
            Some(uid) => Ok(self.ctx.get_user(UserId(uid)).await?),
            None => {
                // is it a userid?
                match input.parse::<u64>() {
                    Ok(uid) => Ok(self.ctx.get_user(UserId(uid)).await?),
                    Err(_) => {
                        //nope, must be a partial name
                        Err(Error::ParseError(ParseError::MemberNotFoundByName(
                            "not implemented yet".to_string(),
                        )))
                    }
                }
            }
        }
    }

    pub async fn get_member(&mut self, gid: GuildId) -> Result<Arc<CachedMember>, Error> {
        let input = self.get_next()?;
        let mention = matchers::get_mention(input);
        match mention {
            // we got a mention
            Some(uid) => match self.ctx.cache.member(gid, UserId(uid)).await? {
                Some(member) => Ok(member),
                None => Err(Error::ParseError(ParseError::MemberNotFoundById(uid))),
            },
            None => {
                // is it a userid?
                match input.parse::<u64>() {
                    Ok(uid) => match self.ctx.cache.member(gid, UserId(uid)).await? {
                        Some(member) => Ok(member),
                        None => Err(Error::ParseError(ParseError::MemberNotFoundById(uid))),
                    },
                    Err(_) => {
                        //nope, must be a partial name
                        Err(Error::ParseError(ParseError::MemberNotFoundByName(
                            "not implemented yet".to_string(),
                        )))
                    }
                }
            }
        }
    }

    pub async fn get_user_or(&mut self, alternative: User) -> Result<Arc<User>, Error> {
        if self.has_next() {
            Ok(self.get_user().await?)
        } else {
            Ok(Arc::new(alternative))
        }
    }
}
