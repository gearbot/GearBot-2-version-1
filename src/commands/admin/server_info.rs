use chrono::Utc;
use std::sync::atomic::Ordering;
use twilight::builders::embed::EmbedBuilder;
use twilight::model::id::GuildId;

use crate::core::cache::CachedChannel;
use crate::core::CommandContext;
use crate::utils::{self, ParseError};
use crate::{CommandResult, Error};

pub async fn server_info(mut ctx: CommandContext) -> CommandResult {
    let guild = match ctx.parser.get_next() {
        Some(possible_guild_id) => {
            let guild_id: u64 = match possible_guild_id.parse() {
                Ok(id) => id,
                Err(_) => return Err(ParseError::InvalidGuildID)?,
            };

            // TODO: This should probably have a higher level up method to
            // be in-between the cache, API, Redis, etc.
            ctx.bot_context
                .cache
                .get_guild(&GuildId(guild_id))
                .ok_or(Error::UnknownGuild(guild_id))?
        }
        None => ctx.get_guild()?,
    };

    let mut category_count: u64 = 0;
    let mut text_count: u64 = 0;
    let mut voice_count: u64 = 0;
    {
        let channels_lock = guild.channels.read().unwrap();
        let all_channels = channels_lock.iter().map(|(_, c)| c);

        for channel in all_channels {
            match **channel {
                CachedChannel::VoiceChannel { .. } => voice_count += 1,
                CachedChannel::Category { .. } => category_count += 1,
                CachedChannel::TextChannel { .. } => text_count += 1,
                _ => (),
            }
        }
    }

    let channels_embed_msg = format!(
        r#"
        Categories: {}\n
        Text Channels: {}\n
        Voice Channels: {}\n
    "#,
        category_count, text_count, voice_count
    );

    let created_at = {
        let timestamp = utils::snowflake_timestamp(guild.id.0);
        utils::age(timestamp, Utc::now(), 2)
    };

    let statuses: (u64, u64, u64, u64) = (0, 0, 0, 0);
    // {
    //     let members = guild.members.read().unwrap();
    //     for member in members.values() {
    //        // TODO: We don't keep track of any user presences.
    //     }
    // };

    let embed = EmbedBuilder::new()
        .add_field("Server Name", &guild.name)
        .inline()
        .commit()
        .add_field("ID", guild.id.0.to_string())
        .inline()
        .commit()
        .add_field("Owner", guild.owner_id.to_string())
        .inline()
        .commit()
        .add_field("Members", guild.member_count.load(Ordering::Relaxed).to_string())
        .inline()
        .commit()
        .add_field("Channels", channels_embed_msg)
        .inline()
        .commit()
        .add_field("Created At", created_at)
        .inline()
        .commit()
        .add_field("VIP Features", guild.features.join(" "))
        .inline()
        .commit()
        .add_field("Server Icon", "TODO: Icon")
        .inline()
        .commit()
        .add_field("Roles", guild.roles.read().unwrap().len().to_string())
        .inline()
        .commit()
        .add_field("Emoji", guild.emoji.len().to_string())
        .inline()
        .commit()
        .add_field("Statuses", format!("{:?}", statuses))
        .inline()
        .commit();

    let embed = match &guild.splash {
        Some(i_url) => embed.image(i_url),
        None => match &guild.discovery_splash {
            Some(i_url) => embed.image(i_url),
            None => embed,
        },
    };

    ctx.reply_raw_with_embed(format!("Information about {}", guild.name), embed.build())
        .await?;

    Ok(())
}
