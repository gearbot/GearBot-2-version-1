use dashmap::{DashMap, ElementGuard};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use twilight::model::channel::permission_overwrite::{
    PermissionOverwrite, PermissionOverwriteType,
};
use twilight::model::channel::Channel;
use twilight::model::guild::{
    DefaultMessageNotificationLevel, Permissions, PremiumTier, VerificationLevel,
};
use twilight::model::id::{ChannelId, EmojiId, GuildId, RoleId, UserId};
use twilight::model::user::UserFlags;
const NO_PERMISSIONS: &[PermissionOverwrite] = &[];

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedGuild {
    // api fields
    pub id: GuildId,
    pub name: String,
    pub icon: Option<String>,
    pub splash: Option<String>,
    pub discovery_splash: Option<String>,
    pub owner_id: UserId,
    pub region: String,
    //can technically be an enum but that will fail as soon as they add a new region
    pub afk_channel_id: Option<ChannelId>,
    pub afk_timeout: u64,
    pub verification_level: VerificationLevel,
    pub default_message_notifications: DefaultMessageNotificationLevel,
    pub roles: DashMap<RoleId, CachedRole>,
    pub emoji: Vec<Arc<CachedEmoji>>,
    pub features: Vec<String>,
    //same as region, will cause issues when they add one
    pub unavailable: bool,
    pub members: DashMap<UserId, Arc<CachedMember>>,
    pub channels: DashMap<ChannelId, Arc<CachedChannel>>,
    //use our own version, easier to work with then twilight's enum
    pub max_presences: Option<u64>,
    //defaults to 25000 if null in the guild create
    pub max_members: Option<u64>,
    // should always be present in guild create, but option just in case
    pub description: Option<String>,
    pub banner: Option<String>,
    pub premium_tier: PremiumTier,
    pub premium_subscription_count: u64,
    pub preferred_locale: String,

    //own fields
    pub complete: bool,
    pub member_count: AtomicU64, //own field because we do not rely on the guild create info for this but rather the
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedRole {
    pub id: RoleId,
    pub name: String,
    pub color: u32,
    pub hoisted: bool,
    pub position: i64,
    pub permissions: Permissions,
    pub managed: bool,
    pub mentionable: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedEmoji {
    pub id: EmojiId,
    pub name: String,
    //only null in reaction objects, but we can fetch from cache for those
    pub roles: Vec<RoleId>,
    pub created_by: Option<UserId>,
    pub requires_colons: bool,
    pub managed: bool,
    pub animated: bool,
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedMember {
    pub user: Arc<CachedUser>,
    pub nickname: Option<String>,
    pub roles: Vec<RoleId>,
    pub joined_at: Option<String>,
    //TODO: convert to date
    pub boosting_since: Option<String>,
    pub server_deafened: bool,
    pub server_muted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CachedChannel {
    TextChannel {
        id: ChannelId,
        guild_id: GuildId,
        position: i64,
        //should be always present in guild create,
        permission_overrides: Vec<PermissionOverwrite>,
        name: String,
        topic: Option<String>,
        nsfw: bool,
        slowmode: Option<u64>,
        parent_id: Option<ChannelId>,
    },
    DM {
        id: ChannelId,
        //TODO: see what else is relevant here, recipients to find the user this is for?
    },
    VoiceChannel {
        id: ChannelId,
        guild_id: GuildId,
        position: i64,
        //should be always present in guild create,
        permission_overrides: Vec<PermissionOverwrite>,
        name: String,
        bitrate: u64,
        user_limit: Option<u64>,
        parent_id: Option<ChannelId>,
    },
    GroupDM {
        id: ChannelId,
        //TODO: see what else is relevant here
    },
    Category {
        id: ChannelId,
        guild_id: GuildId,
        position: i64,
        //should be always present in guild create,
        permission_overrides: Vec<PermissionOverwrite>,
        name: String,
    },
    AnnouncementsChannel {
        id: ChannelId,
        guild_id: GuildId,
        position: i64,
        //should be always present in guild create,
        permission_overrides: Vec<PermissionOverwrite>,
        name: String,
        parent_id: Option<ChannelId>,
    },
    StoreChannel {
        id: ChannelId,
        guild_id: GuildId,
        position: i64,
        //should be always present in guild create,
        name: String,
        parent_id: Option<ChannelId>,
        permission_overrides: Vec<PermissionOverwrite>, //they might not allow for text, but they do have overrides
    },
}

impl CachedChannel {
    /// returns the channel ID
    /// Note this is different from userid when DMing users
    pub fn get_id(&self) -> &ChannelId {
        match self {
            CachedChannel::TextChannel { id, .. } => id,
            CachedChannel::DM { id } => id,
            CachedChannel::VoiceChannel { id, .. } => id,
            CachedChannel::GroupDM { id } => id,
            CachedChannel::Category { id, .. } => id,
            CachedChannel::AnnouncementsChannel { id, .. } => id,
            CachedChannel::StoreChannel { id, .. } => id,
        }
    }

    ///Returns the guild id
    pub fn get_guild_id(&self) -> Option<&GuildId> {
        match self {
            CachedChannel::TextChannel { guild_id, .. } => Some(guild_id),
            CachedChannel::DM { .. } => None,
            CachedChannel::VoiceChannel { guild_id, .. } => Some(guild_id),
            CachedChannel::GroupDM { .. } => None,
            CachedChannel::Category { guild_id, .. } => Some(guild_id),
            CachedChannel::AnnouncementsChannel { guild_id, .. } => Some(guild_id),
            CachedChannel::StoreChannel { guild_id, .. } => Some(guild_id),
        }
    }

    /// Gets the position of this channel
    /// returns 0 for DM (group) channels
    pub fn get_position(&self) -> &i64 {
        match self {
            CachedChannel::TextChannel { position, .. } => position,
            CachedChannel::DM { .. } => &0,
            CachedChannel::VoiceChannel { position, .. } => position,
            CachedChannel::GroupDM { .. } => &0,
            CachedChannel::Category { position, .. } => position,
            CachedChannel::AnnouncementsChannel { position, .. } => position,
            CachedChannel::StoreChannel { position, .. } => position,
        }
    }

    pub fn get_name(&self) -> &str {
        match self {
            CachedChannel::TextChannel { name, .. } => name,
            CachedChannel::DM { .. } => "",
            CachedChannel::VoiceChannel { name, .. } => name,
            CachedChannel::GroupDM { .. } => "",
            CachedChannel::Category { name, .. } => name,
            CachedChannel::AnnouncementsChannel { name, .. } => name,
            CachedChannel::StoreChannel { name, .. } => name,
        }
    }

    pub fn get_topic(&self) -> &Option<String> {
        match self {
            CachedChannel::TextChannel { topic, .. } => topic,
            CachedChannel::DM { .. } => &None,
            CachedChannel::VoiceChannel { .. } => &None,
            CachedChannel::GroupDM { .. } => &None,
            CachedChannel::Category { .. } => &None,
            CachedChannel::AnnouncementsChannel { .. } => &None,
            CachedChannel::StoreChannel { .. } => &None,
        }
    }

    /// get permission overrides
    /// will be empty for
    pub fn get_permission_overrides(&self) -> &[PermissionOverwrite] {
        match self {
            CachedChannel::TextChannel {
                permission_overrides,
                ..
            } => permission_overrides,
            CachedChannel::DM { .. } => NO_PERMISSIONS,
            CachedChannel::VoiceChannel {
                permission_overrides,
                ..
            } => permission_overrides,
            CachedChannel::GroupDM { .. } => NO_PERMISSIONS,
            CachedChannel::Category {
                permission_overrides,
                ..
            } => permission_overrides,
            CachedChannel::AnnouncementsChannel {
                permission_overrides,
                ..
            } => permission_overrides,
            CachedChannel::StoreChannel {
                permission_overrides,
                ..
            } => permission_overrides,
        }
    }

    pub fn is_nsfw(&self) -> &bool {
        match self {
            CachedChannel::TextChannel { nsfw, .. } => nsfw,
            CachedChannel::DM { .. } => &false,
            CachedChannel::VoiceChannel { .. } => &false,
            CachedChannel::GroupDM { .. } => &false,
            CachedChannel::Category { .. } => &false,
            CachedChannel::AnnouncementsChannel { .. } => &false,
            CachedChannel::StoreChannel { .. } => &false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedUser {
    pub id: UserId,
    pub username: String,
    pub discriminator: String,
    pub avatar: Option<String>,
    pub bot_user: bool,
    pub system_user: bool,
    pub public_flags: Option<UserFlags>,
}
