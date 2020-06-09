use super::CommandContext;
use crate::Error;

use std::sync::Arc;

use crate::core::cache::CachedChannel;
use crate::core::{CachedMember, CachedRole, CachedUser};
use crate::utils::CommandError;
use twilight::cache::twilight_cache_inmemory::model as cache_model;
use twilight::model::{
    channel::GuildChannel,
    guild::{Ban, Role},
    id::{ChannelId, RoleId, UserId},
    user::User,
};

impl CommandContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<CachedUser>, Error> {
        self.bot_context.get_user(user_id).await
    }

    pub async fn get_member(&self, user_id: UserId) -> Option<Arc<CachedMember>> {
        match &self.guild {
            Some(g) => self.bot_context.cache.get_member(g.id, user_id),
            None => None,
        }
    }

    pub async fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<CachedChannel>> {
        self.bot_context.cache.get_channel(channel_id)
    }

    pub async fn get_role(&self, role_id: RoleId) -> Option<Arc<CachedRole>> {
        match &self.guild {
            Some(g) => match g.roles.get(&role_id) {
                Some(guard) => Some(guard.value().clone()),
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
}
