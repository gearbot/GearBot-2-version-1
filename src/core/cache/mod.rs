use crate::{gearbot_error, gearbot_info};
use dashmap::DashMap;
use log::debug;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};

pub struct Cache {
    //cache
    guilds: DashMap<GuildId, Arc<CachedGuild>>,
    guild_channels: DashMap<ChannelId, Arc<CachedChannel>>,
    users: DashMap<UserId, Arc<CachedUser>>,
    emoji: DashMap<EmojiId, CachedEmoji>,
    //TODO: handle guild on outage

    //counters
    guild_count: AtomicU64,
    unique_users: AtomicU64,
    total_users: AtomicU64, //is this even possible to get accurate across multiple clusters?
    partial_guilds: AtomicU64,
    filling: AtomicBool,

    //not really required but i like number counters
    role_count: AtomicU64,
    channel_count: AtomicU64,
    emoji_count: AtomicU64,
}

impl Cache {
    pub fn new() -> Self {
        Cache {
            guilds: DashMap::new(),
            guild_channels: DashMap::new(),
            users: DashMap::new(),
            emoji: DashMap::new(),
            guild_count: AtomicU64::new(0),
            unique_users: AtomicU64::new(0),
            total_users: AtomicU64::new(0),
            partial_guilds: AtomicU64::new(0),
            filling: AtomicBool::new(true),
            role_count: AtomicU64::new(0),
            channel_count: AtomicU64::new(0),
            emoji_count: AtomicU64::new(0),
        }
    }

    pub fn update(&self, event: &Event) {
        match event {
            Event::GuildCreate(e) => {
                let mut guild = CachedGuild {
                    id: e.id,
                    name: e.name.clone(),
                    icon: e.icon.clone(),
                    splash: e.splash.clone(),
                    discovery_splash: e.discovery_splash.clone(),
                    owner_id: e.owner_id,
                    region: e.region.clone(),
                    afk_channel_id: e.afk_channel_id,
                    afk_timeout: e.afk_timeout,
                    verification_level: e.verification_level,
                    default_message_notifications: e.default_message_notifications,
                    roles: DashMap::new(),
                    emoji: vec![],
                    features: e.features.clone(),
                    unavailable: false,
                    members: DashMap::new(),
                    channels: DashMap::new(),
                    max_presences: e.max_presences,
                    max_members: e.max_members,
                    description: e.description.clone(),
                    banner: e.banner.clone(),
                    premium_tier: e.premium_tier,
                    premium_subscription_count: e.premium_subscription_count.unwrap_or(0),
                    preferred_locale: e.preferred_locale.clone(),
                    complete: false,
                    member_count: AtomicU64::new(0),
                };

                //handle roles
                for (role_id, role) in e.roles.clone() {
                    let role = CachedRole {
                        id: role_id.clone(),
                        name: role.name.clone(),
                        color: role.color,
                        hoisted: role.hoist,
                        position: role.position,
                        permissions: role.permissions,
                        managed: role.managed,
                        mentionable: role.mentionable,
                    };
                    guild.roles.insert(role_id, role);
                }

                //channels
                for (channel_id, channel) in e.channels.clone() {
                    let (
                        kind,
                        id,
                        position,
                        permission_overrides,
                        name,
                        topic,
                        nsfw,
                        slowmode,
                        parent_id,
                        bitrate,
                        user_limit,
                    ) = match channel {
                        GuildChannel::Category(category) => (
                            category.kind,
                            category.id,
                            category.position,
                            category.permission_overwrites,
                            category.name,
                            None,
                            false,
                            None,
                            None,
                            0,
                            None,
                        ),
                        GuildChannel::Text(text) => (
                            text.kind,
                            text.id,
                            text.position,
                            text.permission_overwrites,
                            text.name,
                            text.topic,
                            text.nsfw,
                            text.rate_limit_per_user,
                            text.parent_id,
                            0,
                            None,
                        ),
                        GuildChannel::Voice(voice) => (
                            voice.kind,
                            voice.id,
                            voice.position,
                            voice.permission_overwrites,
                            voice.name,
                            None,
                            false,
                            None,
                            voice.parent_id,
                            voice.bitrate,
                            voice.user_limit,
                        ),
                    };

                    let channel = match kind {
                        ChannelType::GuildText => CachedChannel::TextChannel {
                            id,
                            guild_id: guild.id,
                            position,
                            permission_overrides,
                            name,
                            topic,
                            nsfw,
                            slowmode,
                            parent_id,
                        },
                        ChannelType::Private => CachedChannel::DM { id },
                        ChannelType::GuildVoice => CachedChannel::VoiceChannel {
                            id,
                            guild_id: guild.id,
                            position,
                            permission_overrides,
                            name,
                            bitrate,
                            user_limit,
                            parent_id,
                        },
                        ChannelType::Group => CachedChannel::GroupDM { id },
                        ChannelType::GuildCategory => CachedChannel::Category {
                            id,
                            guild_id: guild.id,
                            position,
                            permission_overrides,
                            name,
                        },
                        ChannelType::GuildNews => CachedChannel::AnnouncementsChannel {
                            id,
                            guild_id: guild.id,
                            position,
                            permission_overrides,
                            name,
                            parent_id,
                        },
                        ChannelType::GuildStore => CachedChannel::StoreChannel {
                            id,
                            guild_id: guild.id,
                            position,
                            name,
                            parent_id,
                            permission_overrides,
                        },
                    };
                    let ac = Arc::new(channel);
                    self.guild_channels.insert(channel_id, ac.clone());
                    guild.channels.insert(channel_id, ac);
                    self.channel_count.fetch_add(1, Ordering::Relaxed);
                }

                //emoji
                for (emoji_id, emoji) in e.emojis.clone() {
                    let creator = match emoji.user {
                        Some(e) => Some(e.id),
                        None => None,
                    };
                    let emoji = Arc::new(CachedEmoji {
                        id: emoji_id,
                        name: emoji.name,
                        roles: emoji.roles,
                        created_by: creator,
                        requires_colons: emoji.require_colons,
                        managed: emoji.managed,
                        animated: emoji.animated,
                        available: emoji.available,
                    });
                    guild.emoji.push(emoji.clone());
                    self.emoji_count.fetch_add(1, Ordering::Relaxed);
                }

                self.guilds.insert(e.id, Arc::new(guild));
                self.guild_count.fetch_add(1, Ordering::Relaxed);
                let old = self.partial_guilds.fetch_add(1, Ordering::SeqCst);
                debug!("partial_guilds: {}", old);
            }
            Event::MemberChunk(chunk) => {
                match self.get_guild(&chunk.guild_id) {
                    Some(guild) => {
                        for (user_id, member) in chunk.members.clone() {
                            let user = self.get_or_insert_user(member.user);
                            let member = Arc::new(CachedMember {
                                user,
                                nickname: member.nick,
                                roles: member.roles,
                                joined_at: member.joined_at,
                                boosting_since: member.premium_since,
                                server_deafened: member.deaf,
                                server_muted: member.mute,
                            });
                            guild.members.insert(user_id, member);
                        }
                        if (chunk.chunk_count - 1) == chunk.chunk_index {
                            debug!(
                                "Finished processing all chunks for {} ({})",
                                guild.name, guild.id.0
                            );
                            let old = self.partial_guilds.fetch_sub(1, Ordering::SeqCst);
                            // if we where at 1 we are now at 0
                            if old == 1 && self.filling.fetch_and(true, Ordering::Relaxed) {
                                gearbot_important!("Initial cache filling completed!"); //TODO: cluster number
                                self.filling.fetch_or(false, Ordering::SeqCst);
                            }
                        }
                    }
                    None => {
                        gearbot_error!(
                            "Received member chunks for guild {} before it's creation!",
                            chunk.guild_id
                        );
                    }
                }
            }
            _ => {}
        }
    }

    pub fn get_or_insert_user(&self, user: User) -> Arc<CachedUser> {
        match self.get_user(&user.id) {
            Some(user) => user,
            None => {
                let arc = Arc::new(CachedUser {
                    id: user.id,
                    username: user.name,
                    discriminator: user.discriminator,
                    avatar: user.avatar,
                    bot_user: user.bot,
                    system_user: user.system.unwrap_or(false),
                    public_flags: user.public_flags,
                });
                self.users.insert(user.id, arc);
                self.unique_users.fetch_add(1, Ordering::Relaxed);
                self.get_user(&user.id).unwrap().clone()
            }
        }
    }

    pub fn get_guild(&self, guild_id: &GuildId) -> Option<Arc<CachedGuild>> {
        match self.guilds.get(guild_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_channel(&self, channel_id: &ChannelId) -> Option<Arc<CachedChannel>> {
        match self.guild_channels.get(channel_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_user(&self, user_id: &UserId) -> Option<Arc<CachedUser>> {
        match self.users.get(user_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_member(&self, guild_id: &GuildId, user_id: &UserId) -> Option<Arc<CachedMember>> {
        match self.guilds.get(guild_id) {
            Some(guard) => match guard.value().members.get(user_id) {
                Some(guard) => Some(guard.value().clone()),
                None => None,
            },
            None => None,
        }
    }
}

mod structs;
pub use structs::*;

use chrono::format::Numeric::Ordinal;
use std::collections::HashMap;
use std::sync::Arc;
use twilight::gateway::Event;
use twilight::model::channel::Channel::Guild;
use twilight::model::channel::ChannelType;
use twilight::model::channel::GuildChannel;
use twilight::model::user::User;
