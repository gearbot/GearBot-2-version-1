use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight_model::channel::Reaction;
// use twilight_model::id::MessageId;

use crate::cache::{CachedGuild, CachedMember};
use crate::core::bot_context::BotContext;
use crate::core::reactors::{get_emoji, scroll_page};
use crate::core::GuildConfig;
use crate::error::{MessageError, ReactorError};
use crate::translation::{FluArgs, GearBotString};
use crate::utils::Emoji;
use twilight_embed_builder::{EmbedAuthorBuilder, EmbedBuilder, ImageSource};
use twilight_model::channel::embed::Embed;

#[derive(Deserialize, Serialize, Debug)]
pub struct EmojiListReactor {
    pub page: u8,
}

impl EmojiListReactor {
    pub fn processes(&self, reaction: &Reaction) -> Option<Emoji> {
        get_emoji(vec![Emoji::Left, Emoji::Right], reaction)
    }

    pub async fn do_the_thing(
        &mut self,
        emoji: &Emoji,
        ctx: &Arc<BotContext>,
        member: Option<Arc<CachedMember>>,
        reaction: &Reaction,
    ) -> Result<(), ReactorError> {
        if member.is_some() {
            // If we have a cached member, we have a guild id
            if let Some(guild) = ctx.cache.get_guild(&reaction.guild_id.unwrap()).await {
                let pages = guild.emoji.len() as u8 + 1;
                self.page = scroll_page(pages, self.page, &emoji);
                let embed = gen_emoji_page(self.page, pages, &guild, &ctx.get_config(guild.id).await?, ctx).await?;
                ctx.http
                    .update_message(reaction.channel_id, reaction.message_id)
                    .embed(embed)?
                    .await?;
            }
        }

        Ok(())
    }
}

pub async fn gen_emoji_page(
    page: u8,
    pages: u8,
    guild: &Arc<CachedGuild>,
    guild_config: &Arc<GuildConfig>,
    ctx: &Arc<BotContext>,
) -> Result<Embed, MessageError> {
    let lang = &guild_config.language;
    let mut author_builder = EmbedAuthorBuilder::new();
    if let Some(icon_url) = guild.get_icon_url(true) {
        author_builder = author_builder.icon_url(ImageSource::url(icon_url)?)
    }
    Ok(if page == 0 {
        let header_args = FluArgs::with_capacity(1)
            .add("guild_name", guild.name.clone())
            .generate();
        author_builder = author_builder
            //can not panic since server names are only 100 chars long
            .name(ctx.translate_with_args(lang, GearBotString::EmojiOverviewHeader, &header_args))
            .unwrap();
        EmbedBuilder::new()
            .author(author_builder.build())
            .description("TODO: add jumbo image!")
            .unwrap()
            .build()
            .unwrap()
    } else {
        let gear_no = Emoji::No.for_chat();
        let gear_yes = Emoji::Yes.for_chat();

        let header_args = FluArgs::with_capacity(3)
            .add("guild_name", guild.name.clone())
            .add("page", page)
            .add("pages", pages - 1)
            .generate();

        //can not panic since server names are only 100 long
        author_builder =
            author_builder.name(ctx.translate_with_args(lang, GearBotString::EmojiPageHeader, &header_args))?;

        let emoji = guild.emoji.get(page as usize - 1).unwrap();

        let role_info = if emoji.roles.is_empty() {
            gear_no.to_string()
        } else {
            let mut temp = vec![];
            for role in &emoji.roles {
                temp.push(
                    guild
                        .get_role(role)
                        .await
                        .map_or("Unknown role".to_string(), |r| r.name.clone()),
                )
            }
            temp.join(", ")
        };

        let info_arguments = FluArgs::with_capacity(3)
            .add("emoji_name", emoji.name.clone())
            .add("id", emoji.id.to_string())
            .add(
                "requires_colons",
                if emoji.requires_colons { gear_yes } else { gear_no },
            )
            .add("animated", if emoji.animated { gear_yes } else { gear_no })
            .add("managed", if emoji.managed { gear_yes } else { gear_no })
            .add("role_requirement", role_info)
            .generate();

        EmbedBuilder::new()
            .author(author_builder.build())
            .description(ctx.translate_with_args(lang, GearBotString::EmojiInfo, &info_arguments))?
            .image(ImageSource::url(emoji.get_url()).unwrap())
            .build()
            .unwrap()
    })
}
