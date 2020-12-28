use std::sync::Arc;

use twilight_model::guild::Permissions;
use twilight_model::id::{ChannelId, GuildId, UserId};

use super::BotContext;
use crate::cache::{CachedGuild, CachedMember};
use crate::commands::meta::nodes::{CommandNode, GearBotPermissions};
use crate::commands::ROOT_NODE;
use crate::core::guild_config::{GuildConfig, PermissionGroup};
use twilight_model::channel::permission_overwrite::PermissionOverwriteType;

impl BotContext {
    pub async fn get_guild_permissions_for_member(
        &self,
        member: &Arc<CachedMember>,
        guild: &Arc<CachedGuild>,
    ) -> Permissions {
        //owners can do whatever they want
        if guild.owner_id == member.user_id {
            return Permissions::all();
        }

        let mut permissions = Permissions::empty();

        for role_id in &member.roles {
            if let Some(role) = guild.get_role(role_id).await {
                permissions |= role.permissions;
            }
        }
        if permissions.contains(Permissions::ADMINISTRATOR) {
            //admins also can do whatever they want
            Permissions::all()
        } else {
            permissions
        }
    }

    pub async fn get_guild_permissions_for(&self, guild_id: &GuildId, user_id: &UserId) -> Permissions {
        match self.cache.get_guild(guild_id).await {
            Some(guild) => match self.cache.get_member(guild_id, user_id).await {
                Some(member) => self.get_guild_permissions_for_member(&member, &guild).await,
                None => Permissions::empty(),
            },
            None => Permissions::empty(),
        }
    }

    pub async fn get_channel_permissions_for(&self, user_id: UserId, channel_id: ChannelId) -> Permissions {
        if let Some(channel) = self.cache.get_channel(channel_id).await {
            if channel.is_dm() {
                return Permissions::SEND_MESSAGES
                    | Permissions::EMBED_LINKS
                    | Permissions::ATTACH_FILES
                    | Permissions::USE_EXTERNAL_EMOJIS
                    | Permissions::ADD_REACTIONS
                    | Permissions::READ_MESSAGE_HISTORY;
            }

            let mut permissions = self
                .get_guild_permissions_for(&channel.get_guild_id().unwrap(), &user_id)
                .await;
            //admins don't give a **** about overrides
            if permissions.contains(Permissions::ADMINISTRATOR) {
                return Permissions::all();
            }
            if let Some(member) = &self.cache.get_member(&channel.get_guild_id().unwrap(), &user_id).await {
                let overrides = channel.get_permission_overrides();
                let mut everyone_allowed = Permissions::empty();
                let mut everyone_denied = Permissions::empty();
                let mut user_allowed = Permissions::empty();
                let mut user_denied = Permissions::empty();
                let mut role_allowed = Permissions::empty();
                let mut role_denied = Permissions::empty();
                for o in overrides {
                    match o.kind {
                        PermissionOverwriteType::Member(member_id) => {
                            if member_id == user_id {
                                user_allowed |= o.allow;
                                user_denied |= o.deny;
                            }
                        }
                        PermissionOverwriteType::Role(role_id) => {
                            if role_id.0 == channel.get_guild_id().unwrap().0 {
                                everyone_allowed |= o.allow;
                                everyone_denied |= o.deny
                            } else if member.roles.contains(&role_id) {
                                role_allowed |= o.allow;
                                role_denied |= o.deny;
                            }
                        }
                    }
                }

                permissions &= !everyone_denied;
                permissions |= everyone_allowed;

                permissions &= !role_denied;
                permissions |= role_allowed;

                permissions &= !user_denied;
                permissions |= user_allowed;
            };

            permissions
        } else {
            Permissions::empty()
        }
    }

    pub async fn get_permissions_for(
        &self,
        guild: &Arc<CachedGuild>,
        member: &Arc<CachedMember>,
        config: &Arc<GuildConfig>,
    ) -> GearBotPermissions {
        let mut permissions = GearBotPermissions::empty();
        let mut not_negated_denies = GearBotPermissions::empty();

        let discord_permissions = self.get_guild_permissions_for_member(member, guild).await;

        //these are already sorted by priority upon loading
        for group in &config.permission_groups {
            if let Some(perms) = group.discord_perms {
                if discord_permissions.contains(perms) {
                    apply(&mut permissions, &mut not_negated_denies, &group)
                }
            }

            if group.needs_all {
                if group.roles.iter().all(|role_id| member.roles.contains(role_id)) {
                    apply(&mut permissions, &mut not_negated_denies, &group);
                }
            } else if group.roles.iter().any(|role_id| member.roles.contains(role_id)) {
                apply(&mut permissions, &mut not_negated_denies, &group);
            }

            if group.users.iter().any(|user_id| member.user_id == *user_id) {
                apply(&mut permissions, &mut not_negated_denies, &group);
            }
        }

        cascade_groups(&mut permissions, &not_negated_denies);

        self.apply_admin_perms(&member.user_id, &mut permissions);
        permissions
    }

    pub fn apply_admin_perms(&self, user_id: &UserId, permissions: &mut GearBotPermissions) {
        if self.global_admins.contains(user_id) {
            permissions.insert(GearBotPermissions::BOT_ADMIN);
        } else {
            // in theory there is no way this could be set by guild permissions
            // but just in case someone does manage to find a loophole and screw with the bit
            permissions.remove(GearBotPermissions::BOT_ADMIN);
        }
    }
}

fn apply(permissions: &mut GearBotPermissions, not_negated_denies: &mut GearBotPermissions, group: &PermissionGroup) {
    permissions.remove(group.denied_perms);
    permissions.insert(group.granted_perms);

    not_negated_denies.remove(group.granted_perms);
    not_negated_denies.insert(group.denied_perms);
}

fn cascade_groups(permissions: &mut GearBotPermissions, not_negated_denies: &GearBotPermissions) {
    log::trace!(
        "Cascading nodes. permissions: {:?} not negated: {:?}",
        permissions,
        not_negated_denies
    );
    for (g, commands) in ROOT_NODE.by_group.iter() {
        log::trace!("{:?} group is granted, cascading downwards!", g.get_permission());
        for node in commands {
            let denied = not_negated_denies.contains(g.get_permission()) | !permissions.contains(g.get_permission());
            cascade_node(permissions, not_negated_denies, node, !denied);
        }
    }
}

fn cascade_node(
    permissions: &mut GearBotPermissions,
    not_negated_denies: &GearBotPermissions,
    node: &CommandNode,
    parent_available: bool,
) {
    log::trace!(
        "Cascading {}. permissions: {:?}, not negated: {:?}, parent available: {}",
        node.name,
        permissions,
        not_negated_denies,
        parent_available
    );
    // we are denied if we either have an explicit non negated deny (from any group)
    // also when the parent is not available
    // unless we have an explicit grant
    let denied = not_negated_denies.contains(node.command_permission)
        || !parent_available & !permissions.contains(node.command_permission);

    if !denied {
        permissions.insert(node.command_permission)
    }
    let mut any_granted = false;
    for node in &node.node_list {
        cascade_node(permissions, not_negated_denies, node, !denied);
        if permissions.contains(node.command_permission) {
            any_granted = true;
        }
    }
    // we did not have this command, we do have one of it's subcommands and this command does not do anything itself, grant access as it only gives help info
    if denied & !not_negated_denies.contains(node.command_permission) & any_granted & node.handler.is_none() {
        permissions.insert(node.command_permission)
    }
}
