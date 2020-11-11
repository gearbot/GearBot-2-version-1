use crate::cache::CachedUser;
use crate::core::guild_config::{LogCategory, LogStyle};
use crate::core::BotContext;
use crate::error::MessageError;
use crate::translation::{FluArgs, GearBotString};
use crate::utils::Emoji;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use twilight_embed_builder::{EmbedAuthorBuilder, EmbedBuilder, EmbedFooterBuilder, ImageSource};
use twilight_model::channel::embed::Embed;
use twilight_model::id::ChannelId;
use unic_langid::LanguageIdentifier;

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum LogType {
    CommandUsed { command: String },
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum DataLessLogType {
    CommandUsed,
}

impl LogType {
    pub fn get_category(&self) -> LogCategory {
        match self {
            LogType::CommandUsed { .. } => LogCategory::GENERAL,
        }
    }

    pub fn to_embed(
        &self,
        ctx: &Arc<BotContext>,
        lang: &LanguageIdentifier,
        user: &Arc<CachedUser>,
        channel: &Option<ChannelId>,
    ) -> Result<Embed, MessageError> {
        Ok(match self {
            LogType::CommandUsed { command } => {
                let mut command = command.to_string();
                command.truncate(1800);
                EmbedBuilder::new()
                    .description(
                        ctx.translate_with_args(
                            lang,
                            GearBotString::CommandUsedEmbed,
                            &FluArgs::with_capacity(2)
                                .add("channel_id", channel.unwrap().to_string())
                                .add("command", command)
                                .generate(),
                        ),
                    )?
                    .author(
                        EmbedAuthorBuilder::new()
                            .name(user.full_name_with_id())?
                            .url(user.profile_link()),
                    )
                    .thumbnail(ImageSource::url(user.avatar_url())?)
                    .footer(
                        EmbedFooterBuilder::new(ctx.translate(lang, GearBotString::CommandUsedFooter))?
                            .icon_url(ImageSource::url(self.emoji().url())?),
                    )
            }
        }
        .timestamp(chrono::Utc::now().format("%+").to_string())
        .build()?)
    }

    pub fn to_text(
        &self,
        ctx: &Arc<BotContext>,
        lang: &LanguageIdentifier,
        user: &Arc<CachedUser>,
        channel: &Option<ChannelId>,
    ) -> String {
        match self {
            LogType::CommandUsed { command } => {
                let mut command = command.clone();
                command.truncate(1800);
                let args = add_user_args(FluArgs::with_capacity(4), user)
                    .add("command", command.replace("`", "ˋ"))
                    .add("channel_id", channel.unwrap().to_string()); // we always have a channel for command executions

                ctx.translate_with_args(lang, GearBotString::CommandUsedText, &args.generate())
            }
        }
    }

    pub fn emoji(&self) -> Emoji {
        match self {
            LogType::CommandUsed { .. } => Emoji::Online,
        }
    }

    pub fn dataless(&self) -> DataLessLogType {
        match self {
            Self::CommandUsed { .. } => DataLessLogType::CommandUsed,
        }
    }
}

fn add_user_args<'a>(args: FluArgs<'a>, user: &Arc<CachedUser>) -> FluArgs<'a> {
    args.add("name", user.full_name()).add("user_id", user.id.to_string())
}

impl LogStyle {
    pub fn get_fallback(&self) -> Option<Self> {
        match self {
            LogStyle::Text => None,
            LogStyle::Embed => Some(LogStyle::Text),
        }
    }
}
