use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight::model::channel::Reaction;
// use twilight::model::id::MessageId;

use crate::core::cache::{CachedGuild, CachedMember};
use crate::core::reactors::{get_emoji, scroll_page};
use crate::core::BotContext;
use crate::translation::{FluArgs, GearBotString};
use crate::utils::{Emoji, Error};
use twilight::model::channel::embed::Embed;
use twilight_embed_builder::{EmbedAuthorBuilder, EmbedBuilder, ImageSource};

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
        emoji: Emoji,
        ctx: &Arc<BotContext>,
        member: Option<Arc<CachedMember>>,
        reaction: &Reaction,
    ) -> Result<(), Error> {
        if let Some(_) = member {
            //if we have a cached member, we have a guildid
            if let Some(guild) = ctx.cache.get_guild(&reaction.guild_id.unwrap()) {
                let pages = guild.emoji.len() as u8 + 1;
                self.page = scroll_page(pages, self.page, &emoji);
                let embed = gen_emoji_page(self.page, pages, &guild, ctx).await?;
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
    ctx: &Arc<BotContext>,
) -> Result<Embed, Error> {
    let config = ctx.get_config(guild.id).await?;
    let lang = &config.language;
    let mut author_builder = EmbedAuthorBuilder::new();
    if let Some(icon_url) = guild.get_icon_url(true) {
        author_builder = author_builder.icon_url(ImageSource::url(icon_url)?)
    }
    Ok(if page == 0 {
        let header_args = FluArgs::with_capacity(1)
            .insert("guild_name", guild.name.clone())
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
            .insert("guild_name", guild.name.clone())
            .insert("page", page)
            .insert("pages", pages - 1)
            .generate();
        //can not panic since server names are only 100 long
        author_builder =
            author_builder.name(ctx.translate_with_args(lang, GearBotString::EmojiPageHeader, &header_args))?;

        let emoji = guild.emoji.get(page as usize - 1).unwrap();

        let role_info = if emoji.roles.len() == 0 {
            gear_no.to_string()
        } else {
            let temp = emoji
                .roles
                .iter()
                .map(|e| guild.get_role(e).map_or("Unknown role".to_string(), |r| r.name.clone()))
                .collect::<Vec<String>>();
            temp.join(", ")
        };

        let info_arguments = FluArgs::with_capacity(3)
            .insert("emoji_name", emoji.name.clone())
            .insert("id", emoji.id.to_string())
            .insert(
                "requires_colons",
                if emoji.requires_colons { gear_yes } else { gear_no },
            )
            .insert("animated", if emoji.animated { gear_yes } else { gear_no })
            .insert("managed", if emoji.managed { gear_yes } else { gear_no })
            .insert("role_requirement", role_info)
            .generate();

        EmbedBuilder::new()
            .author(author_builder.build())
            .description(ctx.translate_with_args(lang, GearBotString::EmojiInfo, &info_arguments))?
            .image(ImageSource::url(emoji.get_url()).unwrap())
            .build()
            .unwrap()
    })
}
