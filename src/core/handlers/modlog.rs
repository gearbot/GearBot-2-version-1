use std::sync::Arc;

use log::debug;
use twilight::gateway::Event;
use twilight::model::gateway::payload::RequestGuildMembers;

use crate::core::BotContext;
use crate::utils::Error;
use twilight::model::gateway::presence::{ActivityType, Status};

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<BotContext>) -> Result<(), Error> {
    match &event {
        Event::MemberChunk(_chunk) => {}
        Event::UserUpdate(update) => {}
        Event::MemberUpdate(update) => {
            // According to the docs, cache commands can never error, but just to be safe and
            // not spam unwraps everywhere, wrap it.
            // let old_member = ctx.cache.member(update.guild_id, update.user.id).await?;
            //
            // let old_member = match old_member {
            //     Some(om) => om,
            //     None => return Ok(()),
            // };
            //
            // // These cover the possible modlog options
            // // that a member_update could trigger.
            // let old_roles = &old_member.roles;
            // let old_nickname = old_member.nick.as_ref();
            // let old_member = &old_member.user;
            //
            // let new_roles = &update.roles;
            // let new_nickname = update.nick.as_ref();
            // let new_member = &update.user;
            //
            // let mut roles_lost = Vec::new();
            // let mut roles_gained = Vec::new();
            // if new_roles != old_roles {
            //     for old_role in old_roles {
            //         if !new_roles.contains(old_role) {
            //             roles_lost.push(*old_role);
            //         }
            //     }
            //
            //     for new_role in new_roles {
            //         if !old_roles.contains(new_role) {
            //             roles_gained.push(*new_role)
            //         }
            //     }
            // }
            //
            // let username_change = if new_member.name != old_member.name {
            //     Some((&new_member.name, &old_member.name))
            // } else {
            //     None
            // };
            //
            // let nickname_change = match (old_nickname, new_nickname) {
            //     (Some(old_nick), Some(new_nick)) => Some((old_nick, new_nick)),
            //     _ => None,
            // };
            //
            // debug!("A member update occured: ");
            //
            // if let Some((old_alias, new_alias)) = nickname_change {
            //     debug!(
            //         "User {} changed their nickname from '{}' to '{}'",
            //         new_member.name, old_alias, new_alias
            //     );
            // }
            //
            // if let Some((new_name, past_name)) = username_change {
            //     debug!("User '{}' changed their name to '{}'", past_name, new_name);
            // }
            //
            // if !roles_lost.is_empty() {
            //     let roles_lost_display = generate_role_display(&roles_lost, &ctx).await?;
            //     debug!(
            //         "User '{}' lost the following roles: {}",
            //         new_member.name, roles_lost_display
            //     );
            // }
            //
            // if !roles_gained.is_empty() {
            //     let roles_gained_display = generate_role_display(&roles_lost, &ctx).await?;
            //     debug!(
            //         "User '{}' gained the following roles: {}",
            //         new_member.name, roles_gained_display
            //     );
            // }
        }

        Event::MessageCreate(msg) => {
            if let Some(guild_id) = msg.guild_id {
                let config = &ctx.get_config(guild_id).await?.message_logs;
                if config.enabled
                    && !config.ignored_users.contains(&msg.author.id.0)
                    && !(config.ignore_bots && msg.author.bot)
                {
                    ctx.insert_message(&msg.0, guild_id).await?;
                }
            }
        }
        Event::GuildCreate(guild) => {
            let c = ctx.cluster.clone();
            let data = RequestGuildMembers::new_all(guild.id, None);
            debug!("Requesting members for guild {}", guild.id);
            let res = c.command(shard_id, &data).await;

            if let Err(e) = res {
                return Err(Error::TwilightCluster(e));
            }
        }
        _ => {}
    }
    Ok(())
}
