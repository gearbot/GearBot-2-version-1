use std::collections::HashMap;
use std::sync::Arc;

use fluent_bundle::FluentArgs;
use twilight::gateway::shard::Information;
use twilight::model::channel::embed::Embed;
use twilight::model::channel::message::{MessageFlags, MessageType};
use twilight::model::channel::Attachment;
use twilight::model::{id::MessageId, user::CurrentUser};

use crate::commands::meta::nodes::GearBotPermissions;
use crate::core::cache::{CachedChannel, CachedGuild, CachedMember, CachedUser};
use crate::core::{BotContext, GuildConfig};
use crate::parser::Parser;
use crate::translation::{GearBotString, GuildTranslator};
use crate::utils::CommandError;
use crate::Error;

mod messaging;
mod object_fetcher;
mod permissions;

pub struct CommandMessage {
    pub id: MessageId,
    pub content: String,
    pub author: Arc<CachedUser>,
    pub author_as_member: Option<Arc<CachedMember>>,
    pub channel: Arc<CachedChannel>,
    pub attachments: Vec<Attachment>,
    pub embeds: Vec<Embed>,
    pub flags: Option<MessageFlags>,
    pub kind: MessageType,
    pub mention_everyone: bool,
    pub tts: bool,
}

impl CommandMessage {
    pub fn get_author_as_member(&self) -> Result<Arc<CachedMember>, Error> {
        match &self.author_as_member {
            Some(author_as_member) => Ok(author_as_member.clone()),
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }
}

/// The guild context that is returned inside commands that is specific to each guild, with things like the config,
/// language, etc, set and usable behind wrapper methods for simplicity.
pub struct CommandContext {
    pub translator: Arc<GuildTranslator>,
    pub bot_context: Arc<BotContext>,
    config: Arc<GuildConfig>,
    pub message: CommandMessage,
    pub guild: Option<Arc<CachedGuild>>,
    pub shard: u64,
    pub parser: Parser,
    pub permissions: GearBotPermissions,
}

impl CommandContext {
    pub fn new(
        ctx: Arc<BotContext>,
        config: Arc<GuildConfig>,
        message: CommandMessage,
        guild: Option<Arc<CachedGuild>>,
        shard: u64,
        parser: Parser,
        permissions: GearBotPermissions,
    ) -> Self {
        let translator = ctx.translations.get_translator(&config.language);
        CommandContext {
            translator,
            bot_context: ctx,
            config,
            message,
            guild,
            shard,
            parser,
            permissions,
        }
    }

    pub fn get_cluster_info(&self) -> HashMap<u64, Information> {
        self.bot_context.cluster.info()
    }

    pub fn get_bot_user(&self) -> &CurrentUser {
        &self.bot_context.bot_user
    }

    pub fn translate(&self, key: GearBotString) -> String {
        self.bot_context
            .translations
            .get_text_plain(&self.translator.language, key)
            .to_string()
    }

    pub fn translate_with_args(&self, string_key: GearBotString, args: &FluentArgs<'_>) -> String {
        let guild_lang = &self.translator.language;

        self.bot_context
            .translations
            .get_text_with_args(guild_lang, string_key, args)
            .replace("\\n", "\n")
    }

    pub async fn set_config(&self, new_config: GuildConfig) -> Result<(), Error> {
        // This updates it both in the DB and handles our element guard
        match &self.guild {
            Some(g) => self.bot_context.set_config(g.id, new_config).await,
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }

    pub fn get_config(&self) -> Result<Arc<GuildConfig>, Error> {
        if self.message.channel.is_dm() {
            Err(Error::CmdError(CommandError::NoDM))
        } else {
            Ok(self.config.clone())
        }
    }

    pub fn get_guild(&self) -> Result<Arc<CachedGuild>, Error> {
        match &self.guild {
            Some(guild) => Ok(guild.clone()),
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }
}
