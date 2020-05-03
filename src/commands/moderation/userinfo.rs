use crate::core::Context;
use crate::parser::Parser;
use crate::utils::Emoji;
use crate::utils::{CommandError, Error};
use crate::{utils, CommandResult};
use chrono::{DateTime, Utc};
use log::debug;
use serde::export::TryFrom;
use std::borrow::Borrow;
use std::sync::Arc;
use std::time::Duration;
use twilight::builders::embed::EmbedBuilder;
use twilight::model::channel::Message;
use twilight::model::id::ChannelId;
use twilight::model::user::UserFlags;

pub async fn userinfo(ctx: Arc<Context>, msg: Message, mut parser: Parser) -> CommandResult {
    if msg.guild_id.is_none() {
        return Err(Error::CmdError(CommandError::NoDM));
    }

    let user = parser.get_user().await?;

    //set some things that are the same regardless
    let mut content = "".to_string();

    let mut builder = EmbedBuilder::new();
    let mut author_builder = builder
        .author()
        .name(format!("{}#{}", user.name, user.discriminator));
    if user.avatar.is_some() {
        let avatar = user.avatar.as_ref().unwrap();
        let extension = if avatar.starts_with("a_") {
            "gif"
        } else {
            "png"
        };
        author_builder = author_builder.icon_url(format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}",
            user.id,
            user.avatar.as_ref().unwrap(),
            extension
        ))
    }
    builder = author_builder.commit();

    //add badges
    let flags = match user.public_flags {
        Some(flags) => flags,
        None => {
            // we already know for sure the user will exist
            let user = ctx.http.user(user.id.0).await?.unwrap();
            //TODO insert in cache when possible
            user.public_flags.unwrap()
        }
    };

    if flags.contains(UserFlags::DISCORD_EMPLOYEE) {
        content += Emoji::StaffBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::DISCORD_PARTNER) {
        content += Emoji::PartnerBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::HYPESQUAD_EVENTS) {
        content += Emoji::HypesquadEvents.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::BUG_HUNTER) {
        content += Emoji::BugHunterBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::HOUSE_BRAVERY) {
        content += Emoji::BraveryBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::HOUSE_BRILLIANCE) {
        content += Emoji::BrillianceBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::HOUSE_BALANCE) {
        content += Emoji::BalanceBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::BUG_HUNTER_LEVEL_2) {
        content += Emoji::BugHunterLvl2Badge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::VERIFIED_BOT_DEVELOPER) {
        content += Emoji::VerifiedBotDevBadge.for_chat();
        content += " ";
    }

    if flags.contains(UserFlags::EARLY_SUPPORTER) {
        content += Emoji::EarlySupporterBadge.for_chat();
    }

    content += if user.bot {
        Emoji::Robot.for_chat()
    } else {
        ""
    };

    let created_at = utils::snowflake_timestamp(user.id.0);

    content += &format!(
        "\n**User id**: {}\n**Account created on**: {}\n**Account Age**: {}\n\n",
        user.id,
        created_at.format("%A %d %B %Y (%T)"),
        utils::age(created_at, Utc::now(), 2)
    );

    match ctx
        .cache
        .member(msg.guild_id.unwrap(), user.id)
        .await
        .unwrap()
    {
        Some(member) => {
            if member.roles.first().is_some() {
                let role = member.roles.first().unwrap().clone();
                let cached_role = ctx.cache.role(role).await?.unwrap();
                builder = builder.color(cached_role.color);
                let (joined, ago) = match &member.joined_at {
                    Some(joined) => {
                        let joined = DateTime::from_utc(
                            DateTime::parse_from_str(joined, "%FT%T%.f%z")
                                .unwrap()
                                .naive_utc(),
                            Utc,
                        );
                        (
                            joined.format("%A %d %B %Y (%T)").to_string(),
                            utils::age(joined, Utc::now(), 2),
                        )
                    }
                    None => ("Unknown".to_string(), "Unknown".to_string()),
                };

                let mut count = 0;
                let mut roles = "".to_string();
                for role in &member.roles {
                    if count > 0 {
                        roles += ", ";
                    }
                    roles += &format!("<@&{}>", role.0);
                    count += 1;
                    if count == 3 {
                        roles += &format!(" and {} more", member.roles.len() - 3);
                        break;
                    }
                }

                content += &format!(
                    "**Joined on**: {}\n**Been here for**: {}\n**Roles**:{}",
                    joined, ago, roles
                );
                match &member.premium_since {
                    Some(s) => {
                        let since: DateTime<Utc> = DateTime::from_utc(
                            DateTime::parse_from_str(&*s, "%FT%T%.f%z")
                                .unwrap()
                                .naive_utc(),
                            Utc,
                        );
                        content += &format!("**Boosting this server since**: {}", since);
                    }
                    None => {}
                }
            }
        }
        None => {
            builder = builder.color(0x00cea2);
        }
    }

    builder = builder.description(content);

    ctx.http
        .create_message(msg.channel_id)
        .content(format!("User information about <@!{}>", user.id))
        .embed(builder.build())
        .await?;

    Ok(())
}
