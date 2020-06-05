use super::CommandContext;
use crate::Error;

use std::sync::Arc;

use twilight::cache::twilight_cache_inmemory::model as cache_model;
use twilight::model::{
    channel::GuildChannel,
    guild::{Ban, Role},
    id::{ChannelId, RoleId, UserId},
    user::User,
};

impl CommandContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<User>, Error> {
        self.bot_context.get_user(user_id).await
    }

    pub async fn get_cached_member(
        &self,
        user_id: UserId,
    ) -> Option<Arc<cache_model::CachedMember>> {
        self.bot_context
            .cache
            .member(self.id, user_id)
            .await
            .unwrap()
    }

    pub async fn get_cached_guild_channel(
        &self,
        channel_id: ChannelId,
    ) -> Option<Arc<GuildChannel>> {
        self.bot_context
            .cache
            .guild_channel(channel_id)
            .await
            .unwrap()
    }

    pub async fn get_cached_role(&self, role_id: RoleId) -> Option<Arc<Role>> {
        self.bot_context.cache.role(role_id).await.unwrap()
    }

    pub async fn get_ban(&self, user_id: UserId) -> Result<Option<Ban>, Error> {
        let ban = self.bot_context.http.ban(self.id, user_id).await?;

        Ok(ban)
    }
}
