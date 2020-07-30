use std::sync::Arc;

use twilight::model::{
    guild::Ban,
    id::{ChannelId, RoleId, UserId},
};

use crate::core::cache::CachedChannel;
use crate::core::cache::{CachedMember, CachedRole, CachedUser};
use crate::utils::CommandError;
use crate::Error;

use super::CommandContext;

impl CommandContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<CachedUser>, Error> {
        self.bot_context.get_user(user_id).await
    }

    pub fn get_member(&self, user_id: UserId) -> Option<Arc<CachedMember>> {
        match &self.guild {
            Some(g) => self.bot_context.cache.get_member(g.id, user_id),
            None => None,
        }
    }

    pub fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<CachedChannel>> {
        self.bot_context.cache.get_channel(channel_id)
    }

    pub fn get_role(&self, role_id: RoleId) -> Option<Arc<CachedRole>> {
        match &self.guild {
            Some(g) => match g.roles.read().expect("Global role cache got poisoned!").get(&role_id) {
                Some(guard) => Some(guard.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub async fn get_ban(&self, user_id: UserId) -> Result<Option<Ban>, Error> {
        match &self.guild {
            Some(g) => Ok(self.bot_context.http.ban(g.id, user_id).await?),
            None => Err(Error::CmdError(CommandError::NoDM)),
        }
    }

    pub async fn get_dm_for_author(&self) -> Result<Arc<CachedChannel>, Error> {
        self.get_dm_for_user(self.message.author.id).await
    }

    pub async fn get_dm_for_user(&self, user_id: UserId) -> Result<Arc<CachedChannel>, Error> {
        match self.bot_context.cache.get_dm_channel_for(user_id) {
            Some(channel) => Ok(channel),
            None => {
                let channel = self.bot_context.http.create_private_channel(user_id).await?;
                Ok(self.bot_context.cache.insert_private_channel(&channel))
            }
        }
    }
}
