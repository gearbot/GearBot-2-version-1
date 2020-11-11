use twilight_model::{
    guild::Permissions,
    id::{ChannelId, UserId},
};

use super::CommandContext;

impl CommandContext {
    pub fn bot_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_bot_guild_permissions().contains(permissions)
    }

    pub fn get_bot_guild_permissions(&self) -> Permissions {
        let bot_user = self.get_bot_user();
        self.get_guild_permissions_for(&bot_user.id)
    }

    pub fn get_guild_permissions_for(&self, user_id: &UserId) -> Permissions {
        if let Some(guild) = &self.guild {
            self.bot_context.get_guild_permissions_for(&guild.id, user_id)
        } else {
            Permissions::empty()
        }
    }

    pub fn get_bot_channel_permissions(&self) -> Permissions {
        self.get_bot_permissions_for_channel(self.message.channel.get_id())
    }

    pub fn get_bot_permissions_for_channel(&self, channel_id: ChannelId) -> Permissions {
        self.bot_context
            .get_channel_permissions_for(self.get_bot_user().id, channel_id)
    }

    pub fn has_channel_permissions(&self, user_id: UserId, channel_id: ChannelId, permissions: Permissions) -> bool {
        self.bot_context
            .get_channel_permissions_for(user_id, channel_id)
            .contains(permissions)
    }

    pub fn bot_has_channel_permissions(&self, permissions: Permissions) -> bool {
        self.bot_has_permissions_in_channel(self.message.channel.get_id(), permissions)
    }

    pub fn bot_has_permissions_in_channel(&self, channel_id: ChannelId, permissions: Permissions) -> bool {
        self.get_bot_permissions_for_channel(channel_id).contains(permissions)
    }

    pub fn get_author_channel_permissions(&self) -> Permissions {
        self.bot_context
            .get_channel_permissions_for(self.message.author.id, self.message.channel.get_id())
    }

    pub fn get_author_guild_permissions(&self) -> Permissions {
        self.get_guild_permissions_for(&self.message.author.id)
    }

    pub fn author_has_channel_permissions(&self, permissions: Permissions) -> bool {
        self.get_author_channel_permissions().contains(permissions)
    }

    pub fn author_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_author_guild_permissions().contains(permissions)
    }
}
