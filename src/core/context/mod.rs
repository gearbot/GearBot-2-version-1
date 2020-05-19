use crate::core::GuildConfig;
use crate::translation::{GearBotStrings, GuildTranslator, Translations};
use crate::utils::LogType;
use crate::EncryptionKey;
use crate::Error;

use aes_gcm::aead::generic_array::GenericArray;
use chrono::{DateTime, Utc};
use dashmap::{DashMap, ElementGuard};
use deadpool_postgres::Pool;
use fluent_bundle::{FluentArgs, FluentValue};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

use twilight::cache::{twilight_cache_inmemory::model as cache_model, InMemoryCache};
use twilight::gateway::{shard::Information, Cluster};
use twilight::http::Client as HttpClient;
use twilight::model::{
    channel::{embed::Embed, permission_overwrite::PermissionOverwriteType, GuildChannel, Message},
    guild::{Ban, Permissions, Role},
    id::{ChannelId, GuildId, MessageId, RoleId, UserId},
    user::{CurrentUser, User},
};

/// The guild context that is returned inside commands that is specific to each guild, with things like the config,
/// language, etc, set and usable behind wrapper methods for simplicity.
pub struct GuildContext {
    pub id: GuildId,
    pub translator: Arc<GuildTranslator>,
    bot_context: Arc<Context>,
    pub config: ElementGuard<GuildId, GuildConfig>,
}

impl GuildContext {
    pub fn new(
        id: GuildId,
        translator: Arc<GuildTranslator>,
        ctx: Arc<Context>,
        config: ElementGuard<GuildId, GuildConfig>,
    ) -> Self {
        GuildContext {
            id,
            translator,
            bot_context: ctx,
            config,
        }
    }

    pub fn get_config(&self) -> &GuildConfig {
        self.config.value()
    }

    pub async fn set_config(&self, new_config: GuildConfig) -> Result<(), Error> {
        // This updates it both in the DB and handles our element guard
        self.bot_context.set_config(self.id, new_config).await
    }

    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<User>, Error> {
        self.bot_context.get_user(user_id).await
    }

    pub async fn get_cached_member<'a>(
        &'a self,
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

    pub async fn send_message(
        &self,
        message: impl Into<String>,
        channel_id: ChannelId,
    ) -> Result<Message, Error> {
        let sent_msg_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(message)
            .await?;

        Ok(sent_msg_handle)
    }

    pub async fn send_embed(&self, embed: Embed, channel_id: ChannelId) -> Result<Message, Error> {
        let sent_embed_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .embed(embed)
            .await?;

        Ok(sent_embed_handle)
    }

    pub async fn send_message_with_embed(
        &self,
        msg: impl Into<String>,
        embed: Embed,
        channel_id: ChannelId,
    ) -> Result<Message, Error> {
        let sent_handle = self
            .bot_context
            .http
            .create_message(channel_id)
            .content(msg)
            .embed(embed)
            .await?;

        Ok(sent_handle)
    }

    pub async fn update_message(
        &self,
        updated_content: impl Into<String>,
        channel_id: ChannelId,
        msg_id: MessageId,
    ) -> Result<Message, Error> {
        let updated_message_handle = self
            .bot_context
            .http
            .update_message(channel_id, msg_id)
            .content(updated_content.into())
            .await?;

        Ok(updated_message_handle)
    }

    pub async fn get_ban(&self, user_id: UserId) -> Result<Option<Ban>, Error> {
        let ban = self.bot_context.http.ban(self.id, user_id).await?;

        Ok(ban)
    }

    pub async fn get_cluster_info(&self) -> HashMap<u64, Information> {
        self.bot_context.cluster.info().await
    }

    pub fn get_bot_user(&self) -> &CurrentUser {
        &self.bot_context.bot_user
    }

    pub fn get_bot_stats(&self) -> &BotStats {
        &self.bot_context.stats
    }

    pub fn translate_with_args<'a>(
        &'a self,
        string_key: GearBotStrings,
        args: &'a FluentArgs<'a>,
    ) -> String {
        let guild_lang = &self.translator.language;

        self.bot_context
            .translations
            .get_text_with_args(guild_lang, string_key, args)
            .to_string()
    }

    // TODO: Make a macro for compile time validation
    pub fn generate_args<'a, P: 'a, T>(&self, arg_mappings: T) -> FluentArgs<'a>
    where
        &'a P: Into<FluentValue<'a>>,
        T: IntoIterator<Item = &'a (&'a str, &'a P)>,
    {
        self.bot_context.translations.generate_args(arg_mappings)
    }

    pub async fn bot_has_guild_permissions(&self, permissions: Permissions) -> bool {
        self.get_bot_guild_permissions().await.contains(permissions)
    }

    pub async fn get_bot_guild_permissions(&self) -> Permissions {
        let bot_user = self.get_bot_user();
        self.get_guild_permissions_for(bot_user.id).await
    }

    pub async fn get_guild_permissions_for(&self, user_id: UserId) -> Permissions {
        let mut permissions = Permissions::empty();

        if let Some(member) = self.get_cached_member(user_id).await {
            for role_id in &member.roles {
                if let Some(role) = self.get_cached_role(*role_id).await {
                    permissions |= role.permissions;
                }
            }
        };
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

pub struct Context {
    pub cache: InMemoryCache,
    pub cluster: Cluster,
    pub http: HttpClient,
    pub stats: BotStats,
    pub status_type: RwLock<u16>,
    pub status_text: RwLock<String>,
    pub bot_user: CurrentUser,
    configs: DashMap<GuildId, GuildConfig>,
    pub pool: Pool,
    pub translations: Translations,
    __static_master_key: Option<Vec<u8>>,
    log_pumps: DashMap<GuildId, UnboundedSender<(DateTime<Utc>, LogType)>>,
}

impl Context {
    pub fn new(
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient,
        bot_user: CurrentUser,
        pool: Pool,
        translations: Translations,
        key: Option<Vec<u8>>,
    ) -> Self {
        Context {
            cache,
            cluster,
            http,
            stats: BotStats::default(),
            status_type: RwLock::new(3),
            status_text: RwLock::new(String::from("the commands turn")),
            bot_user,
            configs: DashMap::new(),
            pool,
            translations,
            __static_master_key: key,
            log_pumps: DashMap::new(),
        }
    }

    /// Returns if a message was sent by us.
    pub fn is_own(&self, other: &Message) -> bool {
        self.bot_user.id == other.author.id
    }

    fn __get_master_key(&self) -> Option<&EncryptionKey> {
        if let Some(mk_bytes) = &self.__static_master_key {
            let key = GenericArray::from_slice(mk_bytes);
            Some(key)
        } else {
            None
        }
    }
}

mod cache;
mod database;
mod logpump;

mod stats;
pub use stats::BotStats;
