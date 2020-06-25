use chrono::{DateTime, Utc};
use twilight::builders::embed::EmbedBuilder;
use twilight::model::guild::Permissions;
use twilight::model::user::UserFlags;

use crate::core::CommandContext;
use crate::parser::Parser;
use crate::translation::{FluArgs, GearBotString};
use crate::utils::Emoji;
use crate::utils::{CommandError, Error};
use crate::{utils, CommandResult};

const USER_INFO_COLOR: u32 = 0x00_cea2;

pub async fn userinfo(ctx: CommandContext, mut parser: Parser) -> CommandResult {
    if ctx.guild.is_none() {
        return Err(Error::CmdError(CommandError::NoDM));
    }

    let user = parser.get_user_or(ctx.message.author.clone()).await?;

    //set some things that are the same regardless
    let mut content = "".to_string();

    let mut builder = EmbedBuilder::new();
    let mut author_builder = builder
        .author()
        .name(format!("{}#{}", user.username, user.discriminator));

    if let Some(avatar) = user.avatar.as_ref() {
        let extension = if avatar.starts_with("a_") { "gif" } else { "png" };

        author_builder = author_builder.icon_url(format!(
            "https://cdn.discordapp.com/avatars/{}/{}.{}",
            user.id,
            user.avatar.as_ref().unwrap(),
            extension
        ));
    }

    builder = author_builder.commit();

    //add badges
    let flags = match &user.public_flags {
        Some(flags) => flags.clone(),
        None => {
            // we already know for sure the user will exist
            let user = ctx.get_user(user.id).await?;
            ctx.bot_context.cache.update_user(user.clone());
            user.public_flags.unwrap_or(UserFlags::empty())
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

    content += if user.bot_user { Emoji::Robot.for_chat() } else { "" };

    let created_at = utils::snowflake_timestamp(user.id.0);

    content += &format!(
        "\n**User id**: {}\n**Account created on**: {}\n**Account Age**: {}\n\n",
        user.id,
        created_at.format("%A %d %B %Y (%T)"),
        utils::age(created_at, Utc::now(), 2)
    );

    let cached_member = ctx.get_member(user.id);

    match cached_member {
        Some(member) => {
            if let Some(role) = member.roles.first() {
                // This role has to exist
                let cached_role = ctx.get_role(*role).unwrap();

                builder = builder.color(cached_role.color);

                let (joined, ago) = match &member.joined_at {
                    Some(joined) => {
                        let joined = DateTime::from_utc(
                            DateTime::parse_from_str(joined, "%FT%T%.f%z").unwrap().naive_utc(),
                            Utc,
                        );
                        (
                            joined.format("%A %d %B %Y (%T)").to_string(),
                            utils::age(joined, Utc::now(), 2),
                        )
                    }
                    None => ("Unknown".to_string(), "Unknown".to_string()),
                };

                let mut roles = "".to_string();
                for (count, role) in member.roles.iter().enumerate() {
                    if count > 0 {
                        roles += ", ";
                    }

                    roles += &format!("<@&{}>", role.0);

                    if count == 3 {
                        roles += &format!(" and {} more", member.roles.len() - 3);
                        break;
                    }
                }

                content += &format!(
                    "**Joined on**: {}\n**Been here for**: {}\n**Roles**:{}",
                    joined, ago, roles
                );
                if let Some(s) = member.boosting_since.as_ref() {
                    let since: DateTime<Utc> =
                        DateTime::from_utc(DateTime::parse_from_str(s, "%FT%T%.f%z").unwrap().naive_utc(), Utc);
                    content += &format!("**Boosting this server since**: {}", since);
                }
            }
        }
        None => {
            builder = builder.color(USER_INFO_COLOR);
        }
    }

    let bot_has_guild_permissions =
        ctx.bot_has_guild_permissions(Permissions::BAN_MEMBERS) && ctx.get_ban(user.id).await?.is_some();

    if bot_has_guild_permissions {
        content += &*format!(
            "{} **This user is currently banned from this server**",
            Emoji::Bad.for_chat()
        )
    }

    builder = builder.description(content);

    let args = FluArgs::with_capacity(1)
        .insert("userid", user.id.to_string())
        .generate();

    ctx.reply_with_embed(GearBotString::UserinfoHeader, args, builder.build())
        .await?;

    Ok(())
}
