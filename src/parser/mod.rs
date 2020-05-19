use std::sync::Arc;

use log::{debug, info};
use twilight::model::gateway::payload::MessageCreate;

use crate::commands;
use crate::commands::meta::nodes::CommandNode;
use crate::core::{Context, GuildContext};
use crate::utils::{matchers, Error, ParseError};
use crate::utils::{CommandError, Emoji};

use twilight::cache::twilight_cache_inmemory::model::CachedMember;
use twilight::model::channel::GuildChannel;
use twilight::model::channel::Message;
use twilight::model::guild::Permissions;
use twilight::model::id::{ChannelId, GuildId, MessageId, UserId};
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

                let guild_context =
                    generate_guild_context(ctx.clone(), message.guild_id.unwrap()).await?;

                p.index += command_nodes.len();
                let channel_id = message.channel_id;
                let result = node.execute(guild_context, message.0, p).await;
                match result {
                    Ok(_) => Ok(()),
                    Err(error) => match error {
                        Error::ParseError(e) => {
                            ctx.http
                                .create_message(channel_id)
                                .content(format!(
                                    "{} Something went wrong trying to parse that: {}",
                                    Emoji::No.for_chat(),
                                    e
                                ))
                                .await?;
                            Ok(())
                        }
                        Error::CmdError(e) => {
                            ctx.http
                                .create_message(channel_id)
                                .content(format!("{} {}", Emoji::No.for_chat(), e))
                                .await?;
                            Ok(())
                        }
                        e => {
                            ctx.http.create_message(channel_id)
                                .content(format!("{} Something went very wrong trying to execute that command, please try again later or report this on the support server {}", Emoji::Bug.for_chat(), Emoji::Bug.for_chat()))
                                .await?;
                            Err(e)
                        }
                    },
                }?;
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

    pub fn get_remaining(&mut self) -> String {
        self.parts[self.index..self.parts.len()].join(" ")
    }

    pub fn has_next(&self) -> bool {
        self.index < self.parts.len()
    }

    /// Parses what comes next as discord user
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

    pub async fn get_message(&mut self, requester: UserId) -> Result<Message, Error> {
        let input = self.get_next()?;

        // We got an id, get the info from the database
        let message_id = input.parse::<u64>().map_err(|_| CommandError::NoDM)?;

        let channel_id = self
            .ctx
            .get_channel_for_message(message_id)
            .await?
            .ok_or(ParseError::UnknownMessage)?;

        let channel = self
            .ctx
            .cache
            .guild_channel(ChannelId(channel_id))
            .await
            .unwrap()
            .ok_or(ParseError::UnknownChannel(channel_id))?;

        // No DMs here
        let guild_id = self.guild_id.unwrap();
        let guild_ctx = generate_guild_context(self.ctx.clone(), guild_id).await?;

        info!("{:?}", channel);
        match &*channel {
            //TODO: Figure out the twilight mess of guild channel types
            GuildChannel::Category(channel) => {
                let bot_has_access = guild_ctx
                    .bot_has_channel_permissions(
                        channel.id,
                        Permissions::VIEW_CHANNEL & Permissions::READ_MESSAGE_HISTORY,
                    )
                    .await;

                // Verify if the bot has access
                if bot_has_access {
                    let user_has_access = guild_ctx
                        .has_channel_permissions(
                            requester,
                            channel.id,
                            Permissions::VIEW_CHANNEL & Permissions::READ_MESSAGE_HISTORY,
                        )
                        .await;

                    // Verify if the user has access
                    if user_has_access {
                        // All good, fetch the message from the api instead of cache to make sure it's not only up to date but still actually exists
                        let result = self
                            .ctx
                            .http
                            .message(channel.id, MessageId(message_id))
                            .await;

                        match result {
                            Ok(message) => Ok(message.unwrap()),
                            Err(error) => {
                                if error.to_string().contains("status: 404") {
                                    Err(Error::ParseError(ParseError::UnknownMessage))
                                } else {
                                    Err(Error::TwilightHttp(error))
                                }
                            }
                        }
                    } else {
                        Err(Error::ParseError(ParseError::NoChannelAccessUser(
                            channel.name.clone(),
                        )))
                    }
                } else {
                    Err(Error::ParseError(ParseError::NoChannelAccessBot(
                        channel.name.clone(),
                    )))
                }
            }
            _ => unreachable!(),
        }
    }
}

async fn generate_guild_context(
    bot_context: Arc<Context>,
    guild_id: GuildId,
) -> Result<GuildContext, Error> {
    let config = bot_context.get_config(guild_id).await?;

    let lang = &config.language;
    let translator = bot_context.translations.get_translator(lang);

    Ok(GuildContext::new(guild_id, translator, bot_context, config))
}
