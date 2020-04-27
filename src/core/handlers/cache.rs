use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;
use twilight::model::{gateway::payload::RequestGuildMembers, id::RoleId};

use crate::core::Context;
use crate::utils::Error;

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<Context>) -> Result<(), Error> {
    match &event {
        Event::GuildCreate(guild) => {
            ctx.stats.new_guild().await;
            let c = ctx.cluster.clone();
            let data = RequestGuildMembers::new_all(guild.id, None);
            info!("Requesting members for guild {}", guild.id);
            let res = tokio::spawn(async move { c.command(shard_id, &data).await }).await;

            if let Ok(handle) = res {
                match handle {
                    Ok(_) => return Ok(()),
                    Err(e) => return Err(Error::TwilightCluster(e)),
                }
            }
        }
        Event::MemberChunk(_chunk) => {}
        Event::UserUpdate(update) => {
            println!("User update event triggered");
            println!("{:?}", update);
        }
        Event::MemberUpdate(update) => {
            println!("Member update triggered?");
            println!("{:?}", update);
            // According to the docs, cache commands can never error, but just to be safe and
            // not spam unwraps everywhere, wrap it.
            let old_member = ctx.cache.member(update.guild_id, update.user.id).await?;
            // TODO: Figure out why this is always `None`.
            println!("{:?}", old_member);

            let old_member = match old_member {
                Some(om) => om,
                None => return Ok(()),
            };

            // These cover the possible modlog options
            // that a MemberUpdate could trigger.
            let old_roles = &old_member.roles;
            let old_nickname = old_member.nick.as_ref();
            let old_member = &old_member.user;

            let new_roles = &update.roles;
            let new_nickname = update.nick.as_ref();
            let new_member = &update.user;

            let mut roles_lost = Vec::new();
            let mut roles_gained = Vec::new();
            if new_roles != old_roles {
                for old_role in old_roles {
                    if !new_roles.contains(old_role) {
                        roles_lost.push(*old_role);
                    }
                }

                for new_role in new_roles {
                    if !old_roles.contains(new_role) {
                        roles_gained.push(*new_role)
                    }
                }
            }

            let username_change = if new_member.name != old_member.name {
                Some((&new_member.name, &old_member.name))
            } else {
                None
            };

            let nickname_change = match (old_nickname, new_nickname) {
                (Some(old_nick), Some(new_nick)) => Some((old_nick, new_nick)),
                _ => None,
            };

            info!("A member update occured: ");

            if let Some((new_alias, old_alias)) = nickname_change {
                info!(
                    "User {} changed their nicknamename from {} to {}",
                    new_member.name, old_alias, new_alias
                );
            }

            if let Some((new_name, past_name)) = username_change {
                info!("User {} changed their name to {}", past_name, new_name);
            }

            if !roles_lost.is_empty() {
                let roles_lost_display = generate_role_display(&roles_lost, &ctx).await?;
                info!(
                    "User {} lost the following roles: {}",
                    new_member.name, roles_lost_display
                );
            }

            if !roles_gained.is_empty() {
                let roles_gained_display = generate_role_display(&roles_lost, &ctx).await?;
                info!(
                    "User {} gained the following roles: {}",
                    new_member.name, roles_gained_display
                );
            }
        }

        _ => (),
    }
    Ok(())
}

async fn generate_role_display(roles: &[RoleId], ctx: &Context) -> Result<String, Error> {
    let mut display_string = String::new();

    for (pos, role_id) in roles.iter().enumerate() {
        if let Some(role) = ctx.cache.role(*role_id).await? {
            let disp = if pos != roles.len() {
                format!("{}, ", role.name)
            } else {
                format!("{}", role.name)
            };

            display_string.push_str(&disp);
        }
    }

    Ok(display_string)
}
