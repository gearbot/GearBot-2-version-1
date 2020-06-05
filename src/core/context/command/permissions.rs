use super::CommandContext;

use twilight::model::{
    channel::{permission_overwrite::PermissionOverwriteType, GuildChannel},
    guild::Permissions,
    id::{ChannelId, UserId},
};

impl CommandContext {
    pub async fn bot_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_bot_guild_permissions().await.contains(permissions)
    }

    pub async fn get_bot_guild_permissions(&self) -> Permissions {
        let bot_user = self.get_bot_user();
        self.get_guild_permissions_for(bot_user.id).await
    }

    pub async fn get_guild_permissions_for(&self, user_id: UserId) -> Permissions {
        let mut permissions = Permissions::empty();

        // if let Some(member) = self.get_cached_member(user_id).await {
        //     for role_id in &member.roles {
        //         if let Some(role) = self.get_cached_role(*role_id).await {
        //             permissions |= role.permissions;
        //         }
        //     }
        // };
        permissions
    }

    pub async fn get_channel_permissions_for(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
    ) -> Permissions {
        let mut permissions = Permissions::empty();

        if let Some(channel) = self.get_cached_guild_channel(channel_id).await {
            permissions = self.get_guild_permissions_for(user_id).await;
            if let Some(member) = self.get_cached_member(user_id).await {
                let overrides = channel.get_permission_overrides();
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

    pub async fn get_bot_channel_permissions(&self, channel_id: ChannelId) -> Permissions {
        let bot_user = self.get_bot_user();
        self.get_channel_permissions_for(bot_user.id, channel_id)
            .await
    }

    pub async fn has_channel_permissions(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        permissions: Permissions,
    ) -> bool {
        self.get_channel_permissions_for(user_id, channel_id)
            .await
            .contains(permissions)
    }

    pub async fn bot_has_channel_permissions(
        &self,
        channel_id: ChannelId,
        permissions: Permissions,
    ) -> bool {
        self.get_bot_channel_permissions(channel_id)
            .await
            .contains(permissions)
    }
}
