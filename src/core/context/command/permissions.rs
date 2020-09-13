use twilight_model::{
    channel::permission_overwrite::PermissionOverwriteType,
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
        match &self.guild {
            Some(guild) => match self.get_member(user_id) {
                Some(member) => self.bot_context.get_guild_permissions_for(&member, guild),
                None => Permissions::empty(),
            },
            None => Permissions::empty(),
        }
    }

    pub fn get_channel_permissions_for(&self, user_id: UserId, channel_id: ChannelId) -> Permissions {
        let mut permissions = Permissions::empty();

        if let Some(channel) = self.get_channel(channel_id) {
            if channel.is_dm() {
                return Permissions::SEND_MESSAGES
                    | Permissions::EMBED_LINKS
                    | Permissions::ATTACH_FILES
                    | Permissions::USE_EXTERNAL_EMOJIS
                    | Permissions::ADD_REACTIONS
                    | Permissions::READ_MESSAGE_HISTORY;
            }
            permissions = self.get_guild_permissions_for(&user_id);
            //admins don't give a **** about overrides
            if permissions.contains(Permissions::ADMINISTRATOR) {
                return Permissions::all();
            }
            if let Some(member) = &self.message.author_as_member {
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
        };

        permissions
    }

    pub fn get_bot_channel_permissions(&self) -> Permissions {
        self.get_bot_permissions_for_channel(self.message.channel.get_id())
    }

    pub fn get_bot_permissions_for_channel(&self, channel_id: ChannelId) -> Permissions {
        self.get_channel_permissions_for(self.get_bot_user().id, channel_id)
    }

    pub fn has_channel_permissions(&self, user_id: UserId, channel_id: ChannelId, permissions: Permissions) -> bool {
        self.get_channel_permissions_for(user_id, channel_id)
            .contains(permissions)
    }

    pub fn bot_has_channel_permissions(&self, permissions: Permissions) -> bool {
        self.bot_has_permissions_in_channel(self.message.channel.get_id(), permissions)
    }

    pub fn bot_has_permissions_in_channel(&self, channel_id: ChannelId, permissions: Permissions) -> bool {
        self.get_bot_permissions_for_channel(channel_id).contains(permissions)
    }

    pub fn get_author_channel_permissions(&self) -> Permissions {
        self.get_channel_permissions_for(self.message.author.id, self.message.channel.get_id())
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
