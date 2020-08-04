use std::sync::Arc;

use log::{debug, info, trace};

use twilight::model::gateway::payload::MessageCreate;
use twilight::model::id::{GuildId, UserId};

use crate::commands::{
    meta::nodes::{CommandNode, GearBotPermissions},
    ROOT_NODE,
};
use crate::core::cache::{CachedGuild, CachedMember, CachedUser};
use crate::core::{BotContext, CommandContext, CommandMessage, GuildConfig};
use crate::translation::{FluArgs, GearBotString};
use crate::utils::matchers::split_name;
use crate::utils::{matchers, Error, ParseError};
use crate::utils::{CommandError, Emoji};
use lazy_static::lazy_static;
use std::sync::atomic::Ordering;
use twilight::model::guild::Permissions;

lazy_static! {
    static ref BLANK_CONFIG: Arc<GuildConfig> = Arc::new(GuildConfig::default());
}

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
            match part.strip_prefix('"') {
                Some(new_part) => match new_part.strip_suffix('"') {
                    Some(new_part) => part = new_part.to_string(),
                    None => {}
                },
                None => {}
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
    ) -> Result<(), Error> {
        let message = (*message).0;

        //Create parser to process message
        let mut parser = Parser::new(&message.content[prefix.len()..], ctx, shard_id, message.guild_id);
        trace!("Parser processing message: {:?}", &message.content);

        //parse the message to get the nodes
        let command_nodes = parser.get_command();

        //Do we even have a node to execute?
        let node = match command_nodes.last() {
            Some(node) => node,
            None => return Ok(()),
        };

        //assemble the name
        //TODO: do we need this here for anything else then debugging?
        let mut name = String::new();
        for node in command_nodes.iter().skip(1) {
            name += "__";
            name += &node.name
        }

        //grab our own clone of the ctx we can move around
        let ctx = parser.ctx.clone();

        //grab channel info
        let channel_id = message.channel_id;

        let channel = match ctx.cache.get_channel(message.channel_id) {
            Some(channel) => channel,
            None => {
                let err_msg = "Got a message that we do not know the channel for!".to_string();
                return Err(Error::CorruptCacheError(err_msg));
            }
        };

        //author
        let author = match ctx.cache.get_user(message.author.id) {
            Some(author) => author,
            None => {
                return Err(Error::CorruptCacheError(String::from(
                    "Got a message with a command from a user that is not in the cache!",
                )));
            }
        };

        //get optional guild and member, as well as a config and calculate user permissions
        let (guild, member, config, permissions) = if !channel.is_dm() {
            let guild = match ctx.cache.get_guild(&message.guild_id.unwrap()) {
                Some(guild) => guild,
                None => {
                    return Err(Error::CorruptCacheError(String::from(
                        "Got a message for a guild channel that isn't cached!",
                    )));
                }
            };
            let member = match ctx.cache.get_member(&guild.id, &message.author.id) {
                Some(member) => member,
                None => return Err(Error::CorruptCacheError(String::from("User missing in cache!"))),
            };

            let config = ctx.get_config(guild.id).await?;

            let permissions = ctx.get_permissions_for(&guild, &member, &config);

            (Some(guild), Some(member), config, permissions)
        } else {
            let mut perms = GearBotPermissions::empty() | BLANK_CONFIG.permission_groups.get(0).unwrap().granted_perms;
            ctx.apply_admin_perms(&message.author.id, &mut perms);
            (None, None, BLANK_CONFIG.clone(), perms)
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

        let context = CommandContext::new(ctx.clone(), config, cmdm, guild, shard_id, parser, permissions);

        //don't execute commands you are not allowed to execute
        if !permissions.contains(node.command_permission) {
            let args = FluArgs::with_capacity(1)
                .insert("gearno", Emoji::No.for_chat())
                .generate();
            context.reply(GearBotString::MissingPermissions, args).await?;
            return Ok(());
        }

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

    /// Returns the next part of a message if the end of the
    /// message hasn't yet been reached.
    pub fn get_next(&mut self) -> Option<&str> {
        if self.index != self.parts.len() {
            let result = &self.parts[self.index];
            self.index += 1;
            Some(result)
        } else {
            None
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
        let input = self.get_next().ok_or(ParseError::MissingArgument)?;
        let mention = matchers::get_mention(input);
        match mention {
            // we got a mention
            Some(uid) => Ok(self.ctx.get_user(UserId(uid)).await?),
            None => {
                // is it a userid?
                match input.parse::<u64>() {
                    Ok(uid) => Ok(self.ctx.get_user(UserId(uid)).await?),
                    Err(_) => {
                        // reverse our get_next and make the member getter deal with it
                        self.index -= 1;
                        Ok(self.get_member()?.user(&self.ctx.cache))
                    }
                }
            }
        }
    }

    pub fn get_member(&mut self) -> Result<Arc<CachedMember>, Error> {
        let cache = &self.ctx.clone().cache;
        let guild = self.get_guild()?;
        let input = self.get_next().ok_or(ParseError::MissingArgument)?;
        let mention = matchers::get_mention(input);
        match mention {
            // we got a mention
            Some(uid) => match self.ctx.cache.get_member(&guild.id, &UserId(uid)) {
                Some(member) => Ok(member),
                None => Err(Error::ParseError(ParseError::MemberNotFoundById(uid))),
            },
            None => {
                // is it a userid?
                match input.parse::<u64>() {
                    Ok(uid) => match self.ctx.cache.get_member(&guild.id, &UserId(uid)) {
                        Some(member) => Ok(member),
                        None => Err(Error::ParseError(ParseError::MemberNotFoundById(uid))),
                    },
                    Err(_) => {
                        //nope, might be a (partial) name

                        //remove @ if there is one at the start
                        let to_search = if input.starts_with("@") {
                            input
                                .char_indices()
                                .nth(1)
                                .and_then(|(i, _)| input.get(i..))
                                .unwrap_or("")
                        } else {
                            input
                        };
                        let mut name = to_search;
                        let mut discriminator = None;
                        if let Some((n, d)) = split_name(to_search) {
                            name = n;
                            discriminator = Some(d);
                        }
                        let mut matches = vec![];

                        //grab user lock so we don't need to grab one each iteration
                        let users = cache.users.read().expect("Global user cache got poisoned");

                        //loop over the members and collect those that match
                        //some duplicated cause hashmaps don't give value only iterators
                        let members = guild.members.read().expect("Guild inner members cache got poisoned!");
                        for member in members.values() {
                            //if we have a discriminator we have a full name, don't accept partials
                            //note that this does not mean there can only be 1 match as # is valid for nicknames (but not usernames)
                            if let Some(nickname) = &member.nickname {
                                if nickname.starts_with(to_search) {
                                    matches.push(member);
                                }
                            }
                            let user = match users.get(&member.user_id) {
                                Some(user) => user,
                                None => {
                                    return Err(Error::CorruptCacheError("member without cached username".to_string()))
                                }
                            };
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

                        // if we only have one match or work here is done
                        if matches.len() == 1 {
                            Ok(matches.remove(0).clone())
                        } else if matches.len() > 1 {
                            Err(Error::ParseError(ParseError::MultipleMembersByName(input.to_string())))
                        } else {
                            Err(Error::ParseError(ParseError::MemberNotFoundByName(input.to_string())))
                        }
                    }
                }
            }
        }
    }

    fn get_guild_id(&self) -> Result<GuildId, Error> {
        match self.guild_id {
            Some(guild_id) => Ok(guild_id),
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }

    fn get_guild(&self) -> Result<Arc<CachedGuild>, Error> {
        match self.ctx.cache.get_guild(&self.get_guild_id()?) {
            Some(guild) => Ok(guild),
            None => Err(Error::CorruptCacheError(
                "A guild vanished from cache while still parsing a command!".to_string(),
            )),
        }
    }

    pub async fn get_user_or(&mut self, alternative: Arc<CachedUser>) -> Result<Arc<CachedUser>, Error> {
        if self.has_next() {
            Ok(self.get_user().await?)
        } else {
            Ok(alternative)
        }
    }

    pub fn get_member_or(&mut self, alternative: Arc<CachedMember>) -> Result<Arc<CachedMember>, Error> {
        if self.has_next() {
            Ok(self.get_member()?)
        } else {
            Ok(alternative)
        }
    }
}
