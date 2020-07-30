use std::sync::Arc;

use log::{debug, info, trace};

use twilight::model::gateway::payload::MessageCreate;
use twilight::model::id::{GuildId, UserId};

use crate::commands::{
    meta::nodes::{CommandNode, GearBotPermission},
    ROOT_NODE,
};
use crate::core::cache::{CachedMember, CachedUser};
use crate::core::{BotContext, CommandContext, CommandMessage};
use crate::translation::{FluArgs, GearBotString};
use crate::utils::{matchers, Error, ParseError};
use crate::utils::{CommandError, Emoji};
use std::sync::atomic::Ordering;
use twilight::model::guild::Permissions;

#[derive(Clone)]
pub struct Parser {
    pub parts: Vec<String>,
    pub index: usize,
    ctx: Arc<BotContext>,
    shard_id: u64,
    guild_id: Option<GuildId>,
}

impl Parser {
    fn new(content: &str, ctx: Arc<BotContext>, shard_id: u64, guild_id: Option<GuildId>) -> Self {
        Parser {
            parts: content.split_whitespace().map(String::from).collect::<Vec<String>>(),
            index: 0,
            ctx,
            shard_id,
            guild_id,
        }
    }

    pub fn get_command(&mut self) -> Vec<Arc<CommandNode>> {
        let mut nodes = vec![];
        let mut to_search = &ROOT_NODE.all_commands;
        while self.index < self.parts.len() {
            let target = &self.parts[self.index];

            match to_search.get(target) {
                Some(node) => {
                    to_search = &node.sub_nodes;
                    debug!("Found a command node: {}", node.name);
                    self.index += 1;
                    nodes.push(node.clone());
                }
                None => break,
            }
        }
        nodes
    }

    pub async fn figure_it_out(
        prefix: &str,
        message: Box<MessageCreate>,
        ctx: Arc<BotContext>,
        shard_id: u64,
    ) -> Result<(), Error> {
        let message = (*message).0;

        let mut parser = Parser::new(&message.content[prefix.len()..], ctx, shard_id, message.guild_id);
        trace!("Parser processing message: {:?}", &message.content);

        let command_nodes = parser.get_command();

        let mut name = String::new();
        for node in command_nodes.iter().skip(1) {
            name += "__";
            name += &node.name
        }

        let node = match command_nodes.last() {
            Some(node) => node,
            None => return Ok(()),
        };

        let ctx = parser.ctx.clone();

        // TODO: Verify other permissions
        if (node.command_permission == GearBotPermission::AdminGroup) && !ctx.global_admins.contains(&message.author.id)
        {
            return Err(CommandError::InvalidPermissions.into());
        }

        debug!("Executing command: {}", name);

        let channel_id = message.channel_id;

        let (member, config, guild) = {
            match message.guild_id {
                Some(guild_id) => match ctx.cache.get_member(guild_id, message.author.id) {
                    Some(m) => (
                        Some(m),
                        Some(ctx.get_config(guild_id).await?),
                        Some(ctx.cache.get_guild(&guild_id).unwrap()),
                    ),
                    None => {
                        return Err(Error::CorruptCacheError(String::from(
                            "Got a message with a command from someone who is not cached for this guild!",
                        )))
                    }
                },
                None => (None, None, None),
            }
        };

        let channel = match ctx.cache.get_channel(message.channel_id) {
            Some(channel) => channel,
            None => {
                let err_msg = "Got a message that we do not know the channel for!".to_string();
                return Err(Error::CorruptCacheError(err_msg));
            }
        };

        let author = match ctx.cache.get_user(message.author.id) {
            Some(author) => author,
            None => {
                return Err(Error::CorruptCacheError(String::from(
                    "Got a message with a command from a user that is not in the cache!",
                )))
            }
        };

        let cmdm = CommandMessage {
            id: message.id,
            content: message.content,
            author,
            author_as_member: member,
            channel,
            attachments: message.attachments,
            embeds: message.embeds,
            flags: message.flags,
            kind: message.kind,
            mention_everyone: message.mention_everyone,
            tts: message.tts,
        };

        let context = CommandContext::new(ctx.clone(), config, cmdm, guild, shard_id, parser);
        // debug!("Bot channel perms: {:?}", context.get_bot_channel_permissions());
        // debug!("USER channel perms: {:?}", context.get_author_channel_permissions());
        //check if we can send a reply
        if !context.bot_has_channel_permissions(Permissions::SEND_MESSAGES) {
            let msg = &context.message;
            info!(
                "{}#{} ({}) tried to run the {} command in #{} ({}) but I lack send message permissions to execute the command",
                msg.author.username,
                msg.author.discriminator,
                msg.author.id,
                name,
                msg.channel.get_name(),
                msg.channel.get_id()
            );

            let dm_channel = context.get_dm_for_author().await?;

            let key = if context.author_has_channel_permissions(Permissions::MANAGE_CHANNELS) {
                GearBotString::UnableToReplyForManager
            } else {
                GearBotString::UnableToReply
            };

            let args = FluArgs::with_capacity(1)
                .insert("channel", msg.channel.get_name())
                .generate();

            let translated = context.translate_with_args(key, &args);
            // we don't really care if this works or not, nothing we can do if they don't allow DMs from our mutual server(s)
            let _ = ctx.http.create_message(dm_channel.get_id()).content(translated)?.await;
            return Ok(());
        }

        match &node.handler {
            Some(handler) => {
                if let Err(e) = handler(context).await {
                    match e {
                        Error::ParseError(e) => {
                            ctx.http
                                .create_message(channel_id)
                                .content(format!(
                                    "{} Something went wrong trying to parse that: {}",
                                    Emoji::No.for_chat(),
                                    e
                                ))
                                .unwrap()
                                .await?;
                        }
                        Error::CmdError(e) => {
                            ctx.http
                                .create_message(channel_id)
                                .content(format!("{} {}", Emoji::No.for_chat(), e))
                                .unwrap()
                                .await?;
                        }
                        e => {
                            ctx.http.create_message(channel_id)
                                .content(format!("{} Something went very wrong trying to execute that command, please try again later or report this on the support server {}", Emoji::Bug.for_chat(), Emoji::Bug.for_chat())).unwrap()
                                .await?;
                            return Err(e);
                        }
                    }
                }

                ctx.stats.total_command_counts.fetch_add(1, Ordering::Relaxed);
                match ctx.stats.command_counts.get_metric_with_label_values(&[&name]) {
                    Ok(metric) => {
                        metric.inc();
                        Ok(())
                    }
                    Err(e) => Err(Error::PrometheusError(e)),
                }
            }
            None => Ok(()), // TODO: Show help for subcommand
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
    pub async fn get_user(&mut self) -> Result<Arc<CachedUser>, Error> {
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
            Some(uid) => match self.ctx.cache.get_member(gid, UserId(uid)) {
                Some(member) => Ok(member),
                None => Err(Error::ParseError(ParseError::MemberNotFoundById(uid))),
            },
            None => {
                // is it a userid?
                match input.parse::<u64>() {
                    Ok(uid) => match self.ctx.cache.get_member(gid, UserId(uid)) {
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

    pub async fn get_user_or(&mut self, alternative: Arc<CachedUser>) -> Result<Arc<CachedUser>, Error> {
        if self.has_next() {
            Ok(self.get_user().await?)
        } else {
            Ok(alternative)
        }
    }
}
