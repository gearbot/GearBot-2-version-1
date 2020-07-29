use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use twilight::model::guild::{DefaultMessageNotificationLevel, Guild, PartialGuild, PremiumTier, VerificationLevel};
use twilight::model::id::{ChannelId, GuildId, RoleId, UserId};

use crate::core::cache::{Cache, CachedChannel, CachedEmoji, CachedMember, CachedRole};

use super::is_default;
use std::collections::HashMap;

#[derive(Debug)]
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
    pub roles: RwLock<HashMap<RoleId, Arc<CachedRole>>>,
    pub emoji: Vec<Arc<CachedEmoji>>,
    pub features: Vec<String>,
    //same as region, will cause issues when they add one
    pub unavailable: bool,
    pub members: RwLock<HashMap<UserId, Arc<CachedMember>>>,
    pub channels: RwLock<HashMap<ChannelId, Arc<CachedChannel>>>,
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
    pub complete: AtomicBool,
    pub member_count: AtomicU64, //own field because we do not rely on the guild create info for this but rather the
}

impl From<Guild> for CachedGuild {
    fn from(guild: Guild) -> Self {
        let mut cached_guild = CachedGuild {
            id: guild.id,
            name: guild.name,
            icon: guild.icon,
            splash: guild.splash,
            discovery_splash: guild.discovery_splash,
            owner_id: guild.owner_id,
            region: guild.region,
            afk_channel_id: guild.afk_channel_id,
            afk_timeout: guild.afk_timeout,
            verification_level: guild.verification_level,
            default_message_notifications: guild.default_message_notifications,
            roles: RwLock::new(HashMap::new()),
            emoji: vec![],
            features: guild.features,
            unavailable: false,
            members: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            max_presences: guild.max_presences,
            max_members: guild.max_members,
            description: guild.description,
            banner: guild.banner,
            premium_tier: guild.premium_tier,
            premium_subscription_count: guild.premium_subscription_count.unwrap_or(0),
            preferred_locale: guild.preferred_locale,
            complete: AtomicBool::new(false),
            member_count: AtomicU64::new(0),
        };

        //handle roles
        {
            let mut roles = cached_guild
                .roles
                .write()
                .expect("Guild inner roles cache got poisoned!");
            for (role_id, role) in guild.roles {
                roles.insert(role_id, Arc::new(CachedRole::from_role(&role)));
            }
        }

        //channels
        {
            let mut channels = cached_guild
                .channels
                .write()
                .expect("Guild inner channels cache got poisoned!");
            for (channel_id, channel) in guild.channels {
                channels.insert(
                    channel_id,
                    Arc::new(CachedChannel::from_guild_channel(&channel, guild.id)),
                );
            }
        }

        //emoji
        for (_, emoji) in guild.emojis {
            cached_guild.emoji.push(Arc::new(CachedEmoji::from(emoji)));
        }
        cached_guild
    }
}

impl CachedGuild {
    pub fn defrost(cache: &Cache, cold_guild: ColdStorageGuild) -> Self {
        let mut guild = CachedGuild {
            id: cold_guild.id,
            name: cold_guild.name,
            icon: cold_guild.icon,
            splash: cold_guild.splash,
            discovery_splash: cold_guild.discovery_splash,
            owner_id: cold_guild.owner_id,
            region: cold_guild.region,
            afk_channel_id: cold_guild.afk_channel_id,
            afk_timeout: cold_guild.afk_timeout,
            verification_level: cold_guild.verification_level,
            default_message_notifications: cold_guild.default_message_notifications,
            roles: RwLock::new(HashMap::new()),
            emoji: vec![],
            features: vec![],
            unavailable: false,
            members: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            max_presences: cold_guild.max_presences,
            max_members: cold_guild.max_members,
            description: cold_guild.description,
            banner: cold_guild.banner,
            premium_tier: cold_guild.premium_tier,
            premium_subscription_count: cold_guild.premium_subscription_count,
            preferred_locale: cold_guild.preferred_locale,
            complete: AtomicBool::new(true),
            member_count: AtomicU64::new(cold_guild.members.len() as u64),
        };

        {
            let mut roles = guild.roles.write().expect("Guild inner roles cache got poisoned!");
            for role in cold_guild.roles {
                roles.insert(role.id, Arc::new(role));
            }
        }

        {
            let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
            for member in cold_guild.members {
                let user = cache.get_user(&member.user_id).unwrap();
                user.mutual_servers.fetch_add(1, Ordering::SeqCst);
                members.insert(member.user_id, Arc::new(member));
            }
        }

        {
            let mut channels = guild
                .channels
                .write()
                .expect("Guild inner channels cache got poisoned!");
            for channel in cold_guild.channels {
                channels.insert(channel.get_id(), Arc::new(channel));
            }
        }
        for emoji in cold_guild.emoji {
            guild.emoji.push(Arc::new(emoji));
        }
        guild
    }

    pub fn update(&self, other: &PartialGuild) -> Self {
        let guild = CachedGuild {
            id: other.id,
            name: other.name.clone(),
            icon: other.icon.clone(),
            splash: other.splash.clone(),
            discovery_splash: other.discovery_splash.clone(),
            owner_id: other.owner_id,
            region: other.region.clone(),
            afk_channel_id: other.afk_channel_id,
            afk_timeout: other.afk_timeout,
            verification_level: other.verification_level,
            default_message_notifications: other.default_message_notifications,
            roles: RwLock::new(HashMap::new()),
            emoji: self.emoji.clone(),
            features: other.features.clone(),
            unavailable: false,
            members: RwLock::new(HashMap::new()),
            channels: RwLock::new(HashMap::new()),
            max_presences: other.max_presences,
            max_members: other.max_members,
            description: other.description.clone(),
            banner: other.banner.clone(),
            premium_tier: other.premium_tier,
            premium_subscription_count: other.premium_subscription_count.unwrap_or(0),
            preferred_locale: other.preferred_locale.clone(),
            complete: AtomicBool::new(self.complete.load(Ordering::SeqCst)),
            member_count: AtomicU64::new(self.member_count.load(Ordering::SeqCst)),
        };

        {
            let mut roles = guild.roles.write().expect("Guild inner roles cache got poisoned!");
            for role in other.roles.values() {
                roles.insert(role.id, Arc::new(CachedRole::from_role(role)));
            }
        }

        {
            let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
            for guard in self
                .members
                .read()
                .expect("Guild inner members cache got poisoned!")
                .values()
            {
                members.insert(guard.user_id, guard.clone());
            }
        }

        {
            let mut channels = guild
                .channels
                .write()
                .expect("Guild inner channels cache got poisoned!");
            for guard in self
                .channels
                .read()
                .expect("Guild inner channels cache got poisoned!")
                .values()
            {
                channels.insert(guard.get_id(), guard.clone());
            }
        }

        guild
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColdStorageGuild {
    #[serde(rename = "a")]
    pub id: GuildId,
    #[serde(rename = "b")]
    pub name: String,
    #[serde(rename = "c", default, skip_serializing_if = "is_default")]
    pub icon: Option<String>,
    #[serde(rename = "d", default, skip_serializing_if = "is_default")]
    pub splash: Option<String>,
    #[serde(rename = "e", default, skip_serializing_if = "is_default")]
    pub discovery_splash: Option<String>,
    #[serde(rename = "f")]
    pub owner_id: UserId,
    #[serde(rename = "g", default, skip_serializing_if = "is_default")]
    pub region: String,
    //can technically be an enum but that will fail as soon as they add a new region
    #[serde(rename = "h", default, skip_serializing_if = "is_default")]
    pub afk_channel_id: Option<ChannelId>,
    #[serde(rename = "i", default, skip_serializing_if = "is_default")]
    pub afk_timeout: u64,
    #[serde(rename = "j")]
    pub verification_level: VerificationLevel,
    #[serde(rename = "k")]
    pub default_message_notifications: DefaultMessageNotificationLevel,
    #[serde(rename = "l")]
    pub roles: Vec<CachedRole>,
    #[serde(rename = "m")]
    pub emoji: Vec<CachedEmoji>,
    #[serde(rename = "n", default, skip_serializing_if = "is_default")]
    pub features: Vec<String>,
    #[serde(rename = "o")]
    pub members: Vec<CachedMember>,
    #[serde(rename = "p")]
    pub channels: Vec<CachedChannel>,
    #[serde(rename = "q", default, skip_serializing_if = "is_default")]
    pub max_presences: Option<u64>,
    #[serde(rename = "r", default, skip_serializing_if = "is_default")]
    pub max_members: Option<u64>,
    #[serde(rename = "s", default, skip_serializing_if = "is_default")]
    pub description: Option<String>,
    #[serde(rename = "t", default, skip_serializing_if = "is_default")]
    pub banner: Option<String>,
    #[serde(rename = "u", default, skip_serializing_if = "is_default")]
    pub premium_tier: PremiumTier,
    #[serde(rename = "v", default, skip_serializing_if = "is_default")]
    pub premium_subscription_count: u64,
    #[serde(rename = "w", default, skip_serializing_if = "is_default")]
    pub preferred_locale: String,
}

impl From<Arc<CachedGuild>> for ColdStorageGuild {
    fn from(cached_guild: Arc<CachedGuild>) -> Self {
        let guild = cached_guild;
        let mut csg = ColdStorageGuild {
            id: guild.id,
            name: guild.name.clone(),
            icon: guild.icon.clone(),
            splash: guild.splash.clone(),
            discovery_splash: guild.discovery_splash.clone(),
            owner_id: guild.owner_id,
            region: guild.region.clone(),
            afk_channel_id: guild.afk_channel_id,
            afk_timeout: guild.afk_timeout,
            verification_level: guild.verification_level,
            default_message_notifications: guild.default_message_notifications,
            roles: vec![],
            emoji: vec![],
            features: guild.features.clone(),
            members: vec![],
            channels: vec![],
            max_presences: guild.max_presences,
            max_members: guild.max_members,
            description: guild.description.clone(),
            banner: guild.banner.clone(),
            premium_tier: guild.premium_tier,
            premium_subscription_count: guild.premium_subscription_count,
            preferred_locale: guild.preferred_locale.clone(),
        };
        {
            let mut roles = guild.roles.write().expect("Guild inner roles cache got poisoned!");
            for role in roles.values() {
                csg.roles.push(CachedRole::from(role));
            }
            roles.clear();
        }

        for emoji in &guild.emoji {
            csg.emoji.push(emoji.as_ref().clone());
        }

        {
            let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
            for member in members.values() {
                csg.members.push(member.duplicate());
            }
            members.clear();
        }

        for channel in guild
            .channels
            .read()
            .expect("Guild inner channels cache got poisoned!")
            .values()
        {
            csg.channels.push(match channel.as_ref() {
                CachedChannel::TextChannel {
                    id,
                    guild_id,
                    position,
                    permission_overrides,
                    name,
                    topic,
                    nsfw,
                    slowmode,
                    parent_id,
                } => CachedChannel::TextChannel {
                    id: *id,
                    guild_id: *guild_id,
                    position: *position,
                    permission_overrides: permission_overrides.clone(),
                    name: name.clone(),
                    topic: topic.clone(),
                    nsfw: *nsfw,
                    slowmode: *slowmode,
                    parent_id: *parent_id,
                },
                CachedChannel::DM { id, receiver } => CachedChannel::DM {
                    id: *id,
                    receiver: receiver.clone(),
                },
                CachedChannel::VoiceChannel {
                    id,
                    guild_id,
                    position,
                    permission_overrides,
                    name,
                    bitrate,
                    user_limit,
                    parent_id,
                } => CachedChannel::VoiceChannel {
                    id: *id,
                    guild_id: *guild_id,
                    position: *position,
                    permission_overrides: permission_overrides.clone(),
                    name: name.clone(),
                    bitrate: *bitrate,
                    user_limit: *user_limit,
                    parent_id: *parent_id,
                },
                CachedChannel::GroupDM { id, receivers } => CachedChannel::GroupDM {
                    id: *id,
                    receivers: receivers.clone(),
                },
                CachedChannel::Category {
                    id,
                    guild_id,
                    position,
                    permission_overrides,
                    name,
                } => CachedChannel::Category {
                    id: *id,
                    guild_id: *guild_id,
                    position: *position,
                    permission_overrides: permission_overrides.clone(),
                    name: name.clone(),
                },
                CachedChannel::AnnouncementsChannel {
                    id,
                    guild_id,
                    position,
                    permission_overrides,
                    name,
                    parent_id,
                } => CachedChannel::AnnouncementsChannel {
                    id: *id,
                    guild_id: *guild_id,
                    position: *position,
                    permission_overrides: permission_overrides.clone(),
                    name: name.clone(),
                    parent_id: *parent_id,
                },
                CachedChannel::StoreChannel {
                    id,
                    guild_id,
                    position,
                    name,
                    parent_id,
                    permission_overrides,
                } => CachedChannel::StoreChannel {
                    id: *id,
                    guild_id: *guild_id,
                    position: *position,
                    name: name.clone(),
                    parent_id: *parent_id,
                    permission_overrides: permission_overrides.clone(),
                },
            });
        }
        csg
    }
}
