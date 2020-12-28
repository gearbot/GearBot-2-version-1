use std::cmp;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use lazy_static::lazy_static;
use log::{debug, info, trace};
use twilight_model::gateway::payload::MessageCreate;
use twilight_model::guild::Permissions;
use twilight_model::id::{GuildId, UserId};

use crate::cache::{CachedGuild, CachedMember, CachedUser};
use crate::commands::{
    meta::nodes::{CommandNode, GearBotPermissions},
    ROOT_NODE,
};
use crate::core::{BotContext, CommandContext, CommandMessage, GuildConfig};
use crate::error::{CommandError, EventHandlerError, ParseError};
use crate::gearbot_error;
use crate::translation::{FluArgs, GearBotString};
use crate::utils::{matchers, Emoji};

lazy_static! {
    static ref BLANK_CONFIG: Arc<GuildConfig> = Arc::new(GuildConfig::default());
}

pub struct Parser {
    pub parts: Vec<String>,
    index: usize,
    ctx: Arc<BotContext>,
    shard_id: u64,
    guild_id: Option<GuildId>,
}

impl Parser {
    fn new(content: &str, ctx: Arc<BotContext>, shard_id: u64, guild_id: Option<GuildId>) -> Self {
        let temp = content.split_whitespace().collect::<Vec<&str>>();
        let mut parts = vec![];

        let mut index = 0;
        while index < temp.len() {
            let mut part = temp[index].to_string();
            index += 1;
            if part.starts_with('"') && !part.ends_with('"') {
                let mut new_part = part.clone();
                let mut new_index = index;
                while new_index < temp.len() {
                    new_part += " ";
                    new_part += &*temp[new_index];
                    new_index += 1;
                    if new_part.ends_with('"') {
                        index = new_index;
                        part = new_part.clone();
                        break;
                    }
                }
            }

            if let Some(new_part) = part.strip_prefix('"') {
                if let Some(new_part) = new_part.strip_suffix('"') {
                    part = new_part.to_string()
                }
            }

            parts.push(part);
        }

        Parser {
            parts,
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
    ) -> Result<(), EventHandlerError> {
        let message = (*message).0;

        // TODO: This doesn't account for Unicode prefixes
        let mut parser = Parser::new(&message.content[prefix.len()..], ctx, shard_id, message.guild_id);
        trace!("Parser processing message: {:?}", message.content);

        // Parse the message to get the nodes
        let command_nodes = parser.get_command();

        // Is there a valid node to execute?
        let node = match command_nodes.last() {
            Some(node) => node,
            None => return Ok(()),
        };

        // Assemble the command's name
        let mut name = String::new();
        for node in command_nodes.iter().skip(1) {
            name += "__";
            name += &node.name
        }

        let ctx = Arc::clone(&parser.ctx);

        let channel_id = message.channel_id;
        let channel = match ctx.cache.get_channel(channel_id).await {
            Some(channel) => channel,
            None => return Err(EventHandlerError::UnknownChannel(channel_id)),
        };

        let author = match ctx.cache.get_user(message.author.id).await {
            Some(author) => author,
            None => return Err(EventHandlerError::UnknownUser(message.author.id)),
        };

        //get optional guild and member, as well as a config and calculate user permissions
        let (guild, member, config, permissions) = if !channel.is_dm() {
            let guild = match ctx.cache.get_guild(&message.guild_id.unwrap()).await {
                Some(guild) => guild,
                None => return Err(EventHandlerError::UnknownGuild(message.guild_id.unwrap())),
            };

            let member = match ctx.cache.get_member(&guild.id, &message.author.id).await {
                Some(member) => member,
                None => return Err(EventHandlerError::UnknownUser(message.author.id)),
            };

            let config = ctx.get_config(guild.id).await?;

            let permissions = ctx.get_permissions_for(&guild, &member, &config).await;

            (Some(guild), Some(member), config, permissions)
        } else {
            let mut perms = GearBotPermissions::empty() | BLANK_CONFIG.permission_groups[0].granted_perms;
            ctx.apply_admin_perms(&message.author.id, &mut perms);
            (None, None, Arc::clone(&BLANK_CONFIG), perms)
        };

        // Silently ignore any DMs
        // TODO: Maybe return an error?
        let guild = match guild {
            Some(g) => g,
            None => return Ok(()),
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

        let context = CommandContext::new(Arc::clone(&ctx), config, cmdm, guild, shard_id, parser, permissions);

        if !permissions.contains(node.command_permission) {
            let args = FluArgs::with_capacity(1).add("gearno", Emoji::No.for_chat()).generate();
            let _ = context.reply(GearBotString::MissingPermissions, args).await; //ignore result as there is nothing we can do if this fails
            return Ok(());
        }

        //check if we can send a reply
        if !context.bot_has_channel_permissions(Permissions::SEND_MESSAGES).await {
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

            let key = if context
                .author_has_channel_permissions(Permissions::MANAGE_CHANNELS)
                .await
            {
                GearBotString::UnableToReplyForManager
            } else {
                GearBotString::UnableToReply
            };

            let args = FluArgs::with_capacity(1)
                .add("channel", msg.channel.get_name())
                .generate();

            let translated = context.translate_with_args(key, &args);
            // we don't really care if this works or not, nothing we can do if they don't allow DMs from our mutual server(s)
            let _ = ctx
                .http
                .create_message(dm_channel.get_id())
                .content(translated)
                .unwrap()
                .await;

            return Ok(());
        }

        match &node.handler {
            Some(handler) => {
                if let Err(e) = handler(context).await {
                    match e {
                        CommandError::ParseError(e) => {
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
                        CommandError::NoDM | CommandError::InvalidPermissions => {
                            ctx.http
                                .create_message(channel_id)
                                .content(format!("{} {}", Emoji::No.for_chat(), e))
                                .unwrap()
                                .await?;
                        }
                        CommandError::OtherFailure(e) => {
                            ctx.http.create_message(channel_id)
                                .content(format!("{} Something went very wrong trying to execute that command, please try again later or report this on the support server {}", Emoji::Bug.for_chat(), Emoji::Bug.for_chat())).unwrap()
                                .await?;

                            //TODO: better logging
                            gearbot_error!("Command error: {}", e);
                            return Ok(());
                        }
                    }
                }

                ctx.stats.total_command_counts.fetch_add(1, Ordering::Relaxed);
                match ctx.stats.command_counts.get_metric_with_label_values(&[&name]) {
                    Ok(metric) => metric.inc(),
                    Err(e) => log::error!("Failed to increment the command count metric: {}", e),
                }

                Ok(())
            }
            None => Ok(()), // TODO: Show help for subcommand
        }
    }

    pub fn get_next(&mut self) -> Result<&str, ParseError> {
        if self.index == self.parts.len() {
            Err(ParseError::MissingArgument)
        } else {
            let result = &self.parts[self.index];
            self.index += 1;
            Ok(result)
        }
    }

    pub fn get_remaining(&mut self) -> String {
        self.parts[self.index..self.parts.len()].join(" ")
    }

    pub fn has_next(&self) -> bool {
        self.index < self.parts.len()
    }

    async fn get_member(&mut self) -> Result<Arc<CachedMember>, ParseError> {
        let cache = &Arc::clone(&self.ctx).cache;
        let guild = self.get_guild().await?;

        match self.get_affected_user()? {
            Some(id) => cache
                .get_member(&guild.id, &UserId(id))
                .await
                .ok_or(ParseError::MemberNotFoundById(id)),
            None => {
                // Might be a (partial) name
                let input = self.get_next()?;

                //remove @ if there is one at the start
                let to_search = input.trim_start_matches('@');

                let (name, discriminator) = match matchers::split_name(to_search) {
                    Some((name, discrim)) => (name, Some(discrim)),
                    None => (to_search, None),
                };

                let mut matches = vec![];

                let members = guild.members.read().await;
                for member in members.values() {
                    // If we have a discriminator, we have a full name, don't accept partials.
                    // note that this does not mean there can only be 1 match as # is valid for nicknames (but not usernames)
                    if let Some(nickname) = &member.nickname {
                        if nickname.starts_with(to_search) {
                            matches.push(member);
                            // Pass early and don't incur a user lock below.
                            continue;
                        }
                    }

                    let user = member.user(cache).await;
                    match discriminator {
                        Some(discriminator) => {
                            if user.username == name && user.discriminator == discriminator {
                                matches.push(member)
                            }
                        }
                        None => {
                            if user.username.starts_with(name) {
                                matches.push(member);
                            }
                        }
                    }
                }

                match matches.len().cmp(&1) {
                    cmp::Ordering::Equal => Ok(Arc::clone(matches.remove(0))),
                    cmp::Ordering::Greater => Err(ParseError::MultipleMembersByName(input.to_string())),
                    cmp::Ordering::Less => Err(ParseError::MemberNotFoundByName(input.to_string())),
                }
            }
        }
    }

    fn get_affected_user(&mut self) -> Result<Option<u64>, ParseError> {
        let input = self.get_next()?;

        match matchers::get_mention(input) {
            // A user was mentioned
            Some(mention) => Ok(Some(mention)),
            None => {
                // Did they provide a user ID?
                Ok(input.parse().ok())
            }
        }
    }

    fn get_guild_id(&self) -> Result<GuildId, ParseError> {
        match self.guild_id {
            Some(guild_id) => Ok(guild_id),
            None => Err(ParseError::NoDm),
        }
    }

    async fn get_guild(&self) -> Result<Arc<CachedGuild>, ParseError> {
        self.ctx
            .cache
            .get_guild(&self.get_guild_id()?)
            .await
            .ok_or(ParseError::CorruptCache)
    }

    /// Parses what comes next as discord user
    async fn get_user(&mut self) -> Result<Arc<CachedUser>, ParseError> {
        match self.get_affected_user()? {
            Some(id) => Ok(self.ctx.get_user(UserId(id)).await?),
            None => {
                // reverse our get_next and make the member getter deal with it
                self.index -= 1;
                Ok(self.get_member().await?.user(&self.ctx.cache).await)
            }
        }
    }

    pub async fn get_user_or(&mut self, alternative: Arc<CachedUser>) -> Result<Arc<CachedUser>, ParseError> {
        if self.has_next() {
            Ok(self.get_user().await?)
        } else {
            Ok(alternative)
        }
    }

    pub async fn get_member_or(&mut self, alternative: Arc<CachedMember>) -> Result<Arc<CachedMember>, ParseError> {
        if self.has_next() {
            Ok(self.get_member().await?)
        } else {
            Ok(alternative)
        }
    }

    pub fn peek(&self) -> Option<&String> {
        self.parts.get(self.index)
    }
}
