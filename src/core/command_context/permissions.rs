use twilight_model::{
    guild::Permissions,
    id::{ChannelId, UserId},
};

use super::CommandContext;

impl CommandContext {
    pub async fn bot_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_bot_guild_permissions().await.contains(permissions)
    }

    pub async fn get_bot_guild_permissions(&self) -> Permissions {
        let bot_user = self.get_bot_user();
        self.get_guild_permissions_for(&bot_user.id).await
    }

    pub async fn get_guild_permissions_for(&self, user_id: &UserId) -> Permissions {
        self.bot_context
            .get_guild_permissions_for(&self.guild.id, user_id)
            .await
    }

    pub async fn get_bot_channel_permissions(&self) -> Permissions {
        self.get_bot_permissions_for_channel(self.message.channel.get_id())
            .await
    }

    pub async fn get_bot_permissions_for_channel(&self, channel_id: ChannelId) -> Permissions {
        self.bot_context
            .get_channel_permissions_for(self.get_bot_user().id, channel_id)
            .await
    }

    pub async fn has_channel_permissions(
        &self,
        user_id: UserId,
        channel_id: ChannelId,
        permissions: Permissions,
    ) -> bool {
        self.bot_context
            .get_channel_permissions_for(user_id, channel_id)
            .await
            .contains(permissions)
    }

    pub async fn bot_has_channel_permissions(&self, permissions: Permissions) -> bool {
        self.bot_has_permissions_in_channel(self.message.channel.get_id(), permissions)
            .await
    }

    pub async fn bot_has_permissions_in_channel(&self, channel_id: ChannelId, permissions: Permissions) -> bool {
        self.get_bot_permissions_for_channel(channel_id)
            .await
            .contains(permissions)
    }

    pub async fn get_author_channel_permissions(&self) -> Permissions {
        self.bot_context
            .get_channel_permissions_for(self.message.author.id, self.message.channel.get_id())
            .await
    }

    pub async fn get_author_guild_permissions(&self) -> Permissions {
        self.get_guild_permissions_for(&self.message.author.id).await
    }

    pub async fn author_has_channel_permissions(&self, permissions: Permissions) -> bool {
        self.get_author_channel_permissions().await.contains(permissions)
    }

    pub async fn author_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_author_guild_permissions().await.contains(permissions)
    }
}
