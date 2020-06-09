use std::collections::HashMap;
use std::sync::Arc;

use dashmap::ElementGuard;
use fluent_bundle::{FluentArgs, FluentValue};
use twilight::gateway::shard::Information;
use twilight::model::{id::GuildId, user::CurrentUser};

pub use command_message::CommandMessage;

use crate::core::cache::CachedGuild;
use crate::core::context::bot::BotStats;
use crate::core::{BotContext, GuildConfig};
use crate::translation::{GearBotStrings, GuildTranslator, DEFAULT_LANG};
use crate::utils::CommandError;
use crate::Error;

/// The guild context that is returned inside commands that is specific to each guild, with things like the config,
/// language, etc, set and usable behind wrapper methods for simplicity.
pub struct CommandContext {
    pub translator: Arc<GuildTranslator>,
    pub bot_context: Arc<BotContext>,
    config: Option<ElementGuard<GuildId, GuildConfig>>,
    pub message: CommandMessage,
    pub guild: Option<Arc<CachedGuild>>,
}

impl CommandContext {
    pub fn new(
        ctx: Arc<BotContext>,
        config: Option<ElementGuard<GuildId, GuildConfig>>,
        message: CommandMessage,
        guild: Option<Arc<CachedGuild>>,
    ) -> Self {
        let translator = match &config {
            Some(guard) => ctx.translations.get_translator(&guard.value().language),
            None => ctx.translations.get_translator(&DEFAULT_LANG),
        };
        CommandContext {
            translator,
            bot_context: ctx,
            config,
            message,
            guild,
        }
    }

    pub async fn get_cluster_info(&self) -> HashMap<u64, Information> {
        self.bot_context.cluster.info().await
    }

    pub fn get_bot_user(&self) -> &CurrentUser {
        &self.bot_context.bot_user
    }

    pub fn get_bot_stats(&self) -> &BotStats {
        &self.bot_context.stats
    }

    pub fn translate_with_args<'a>(
        &'a self,
        string_key: GearBotStrings,
        args: &'a FluentArgs<'a>,
    ) -> String {
        let guild_lang = &self.translator.language;

        self.bot_context
            .translations
            .get_text_with_args(guild_lang, string_key, args)
            .to_string()
    }

    // TODO: Make a macro for compile time validation
    pub fn generate_args<'a, P: 'a, T>(&self, arg_mappings: T) -> FluentArgs<'a>
    where
        &'a P: Into<FluentValue<'a>>,
        T: IntoIterator<Item = &'a (&'a str, &'a P)>,
    {
        self.bot_context.translations.generate_args(arg_mappings)
    }

    pub async fn set_config(&self, new_config: GuildConfig) -> Result<(), Error> {
        // This updates it both in the DB and handles our element guard
        match &self.guild {
            Some(g) => self.bot_context.set_config(g.id, new_config).await,
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }

    pub fn get_config(&self) -> Result<&GuildConfig, Error> {
        match &self.config {
            Some(guard) => Ok(guard.value()),
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }
}

mod bouncers;
mod command_message;
mod messaging;
mod object_fetcher;
mod permissions;
