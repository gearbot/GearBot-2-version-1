use crate::core::Context;
use twilight::model::channel::permission_overwrite::PermissionOverwriteType;
use twilight::model::channel::GuildChannel;
use twilight::model::guild::Permissions;
use twilight::model::id::{ChannelId, GuildId, UserId};

impl Context {
    pub async fn bot_has_guild_permissions(
        &self,
        guild_id: GuildId,
        permissions: Permissions,
    ) -> bool {
        self.get_bot_guild_permissions(guild_id)
            .await
            .contains(permissions)
    }

    pub async fn get_bot_guild_permissions(&self, guild_id: GuildId) -> Permissions {
        self.get_guild_permissions_for(guild_id, self.bot_user.id)
            .await
    }

    pub async fn get_guild_permissions_for(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Permissions {
        let mut permissions = Permissions::empty();
        if let Some(member) = self.cache.member(guild_id, user_id).await.unwrap() {
            for role_id in &member.roles {
                if let Some(role) = self.cache.role(role_id.clone()).await.unwrap() {
                    permissions |= role.permissions;
                }
            }
        };
        permissions
    }

    pub async fn get_channel_permissions_for(
        &self,
        guild_id: GuildId,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Permissions {
        let mut permissions = self.get_guild_permissions_for(guild_id, user_id).await;
        if let Some(channel) = self.cache.guild_channel(channel_id).await.unwrap() {
            if let Some(member) = self.cache.member(guild_id, user_id).await.unwrap() {
                let overrides = match &*channel {
                    GuildChannel::Category(category) => &category.permission_overwrites,
                    GuildChannel::Text(channel) => &channel.permission_overwrites,
                    GuildChannel::Voice(channel) => &channel.permission_overwrites,
                };
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
                            if member.roles.contains(&role_id) {
                                role_allowed |= o.allow;
                                role_denied |= o.deny;
                            }
                        }
                    }
                }

                permissions &= !role_denied;
                permissions |= role_allowed;

                permissions &= !user_denied;
                permissions |= user_allowed;
            };
        };
        permissions
    }
}
