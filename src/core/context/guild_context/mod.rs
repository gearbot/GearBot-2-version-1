use super::{BotStats, Context, GuildConfig};
use crate::translation::{GearBotStrings, GuildTranslator};
use crate::Error;

use dashmap::ElementGuard;
use fluent_bundle::{FluentArgs, FluentValue};
use std::collections::HashMap;
use std::sync::Arc;

use twilight::gateway::shard::Information;
use twilight::model::{id::GuildId, user::CurrentUser};

/// The guild context that is returned inside commands that is specific to each guild, with things like the config,
/// language, etc, set and usable behind wrapper methods for simplicity.
pub struct GuildContext {
    pub id: GuildId,
    pub translator: Arc<GuildTranslator>,
    bot_context: Arc<Context>,
    pub config: ElementGuard<GuildId, GuildConfig>,
}

impl GuildContext {
    pub fn new(
        id: GuildId,
        translator: Arc<GuildTranslator>,
        ctx: Arc<Context>,
        config: ElementGuard<GuildId, GuildConfig>,
    ) -> Self {
        GuildContext {
            id,
            translator,
            bot_context: ctx,
            config,
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

    pub fn get_config(&self) -> &GuildConfig {
        self.config.value()
    }

    pub async fn set_config(&self, new_config: GuildConfig) -> Result<(), Error> {
        // This updates it both in the DB and handles our element guard
        self.bot_context.set_config(self.id, new_config).await
    }
}

mod messaging;
mod object_fetcher;
mod permissions;
