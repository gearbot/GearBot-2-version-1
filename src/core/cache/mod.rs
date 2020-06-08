use crate::{gearbot_error, gearbot_important, gearbot_info};
use dashmap::DashMap;
use futures::future;
use log::{debug, error};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};

pub struct Cache {
    //cluster info
    cluster_id: u64,

    //cache
    guilds: DashMap<GuildId, Arc<CachedGuild>>,
    guild_channels: DashMap<ChannelId, Arc<CachedChannel>>,
    users: DashMap<UserId, Arc<CachedUser>>,
    emoji: DashMap<EmojiId, CachedEmoji>,
    //TODO: handle guild on outage

    //counters
    guild_count: AtomicU64,
    unique_users: AtomicU64,
    total_users: AtomicU64,
    //is this even possible to get accurate across multiple clusters?
    partial_guilds: AtomicU64,
    filling: AtomicBool,

    //not really required but i like number counters
    role_count: AtomicU64,
    channel_count: AtomicU64,
    emoji_count: AtomicU64,
}

impl Cache {
    pub fn new(cluster_id: u64) -> Self {
        Cache {
            cluster_id,
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
                                "Finished processing all chunks for {} ({}). {:?} guilds to go!",
                                guild.name, guild.id.0, self.partial_guilds
                            );
                            let old = self.partial_guilds.fetch_sub(1, Ordering::SeqCst);
                            // if we where at 1 we are now at 0
                            if old == 1 && self.filling.fetch_and(true, Ordering::Relaxed) {
                                gearbot_important!(
                                    "Initial cache filling completed for cluster {}!",
                                    self.cluster_id
                                );
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

    pub async fn prepare_cold_resume(&self, redis_pool: &ConnectionPool, handlers: usize) {
        //clear global caches so arcs can be cleaned up
        self.guild_channels.clear();
        //let's go to hyperspeed
        let mut tasks = vec![];
        let mut user_tasks = vec![];

        //but not yet, collect their work first before they start sabotaging each other again >.>
        let mut work_orders: Vec<Vec<GuildId>> = vec![vec![]; handlers];
        let mut user_work_orders: Vec<Vec<UserId>> = vec![vec![]; handlers];

        let mut count = 0;
        for guard in self.guilds.iter() {
            work_orders[count % handlers].push(guard.key().clone());
            count += 1;
        }

        debug!("Freezing {:?} guilds", self.guild_count);
        for i in 0..handlers {
            tasks.push(self._prepare_cold_resume_guild(redis_pool, work_orders[i].clone(), i));
        }

        future::join_all(tasks).await;

        count = 0;
        let user_chunks = (self.unique_users.load(Ordering::Relaxed) / 100000 + 1) as usize;
        let mut user_work_orders: Vec<Vec<UserId>> = vec![vec![]; user_chunks];
        for guard in self.users.iter() {
            user_work_orders[count % user_chunks].push(guard.key().clone());
            count += 1;
        }
        debug!("Freezing {:?} users", self.unique_users);
        redis_pool.get().await.set
        for i in 0..user_chunks {
            user_tasks.push(self._prepare_cold_resume_user(redis_pool, user_work_orders[i].clone(), i));
        }

        future::join_all(user_tasks).await;
        self.users.clear();
    }

    async fn _prepare_cold_resume_guild(
        &self,
        redis_pool: &ConnectionPool,
        todo: Vec<GuildId>,
        index: usize,
    ) {
        debug!(
            "Guild dumper {} started freezing {} guilds",
            index,
            todo.len()
        );
        let mut connection = redis_pool.get().await;
        for key in todo {
            let data = {
                let g = self.guilds.remove_take(&key).unwrap();
                let mut csg = ColdStorageGuild {
                    id: g.id,
                    name: g.name.clone(),
                    icon: g.icon.clone(),
                    splash: g.splash.clone(),
                    discovery_splash: g.discovery_splash.clone(),
                    owner_id: g.owner_id,
                    region: g.region.clone(),
                    afk_channel_id: g.afk_channel_id,
                    afk_timeout: g.afk_timeout,
                    verification_level: g.verification_level,
                    default_message_notifications: g.default_message_notifications,
                    roles: vec![],
                    emoji: vec![],
                    features: g.features.clone(),
                    members: vec![],
                    channels: vec![],
                    max_presences: g.max_presences,
                    max_members: g.max_members,
                    description: g.description.clone(),
                    banner: g.banner.clone(),
                    premium_tier: g.premium_tier,
                    premium_subscription_count: g.premium_subscription_count,
                    preferred_locale: g.preferred_locale.clone(),
                };
                for role in &g.roles {
                    csg.roles.push(CachedRole {
                        id: role.id,
                        name: role.name.clone(),
                        color: role.color,
                        hoisted: role.hoisted,
                        position: role.position,
                        permissions: role.permissions,
                        managed: role.managed,
                        mentionable: role.mentionable,
                    })
                }
                g.roles.clear();

                for emoji in &g.emoji {
                    csg.emoji.push(emoji.as_ref().clone());
                }
                for member in &g.members {
                    csg.members.push({
                        ColdStorageMember {
                            id: member.user.id,
                            nickname: member.nickname.clone(),
                            roles: member.roles.clone(),
                            joined_at: member.joined_at.clone(),
                            boosting_since: member.joined_at.clone(),
                            server_deafened: member.server_deafened,
                            server_muted: member.server_muted,
                        }
                    });
                }
                g.members.clear();

                for channel in &g.channels {
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
                            id: id.clone(),
                            guild_id: guild_id.clone(),
                            position: position.clone(),
                            permission_overrides: permission_overrides.clone(),
                            name: name.clone(),
                            topic: topic.clone(),
                            nsfw: nsfw.clone(),
                            slowmode: slowmode.clone(),
                            parent_id: parent_id.clone(),
                        },
                        CachedChannel::DM { id } => CachedChannel::DM { id: id.clone() },
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
                            id: id.clone(),
                            guild_id: guild_id.clone(),
                            position: position.clone(),
                            permission_overrides: permission_overrides.clone(),
                            name: name.clone(),
                            bitrate: bitrate.clone(),
                            user_limit: user_limit.clone(),
                            parent_id: parent_id.clone(),
                        },
                        CachedChannel::GroupDM { id } => CachedChannel::GroupDM { id: id.clone() },
                        CachedChannel::Category {
                            id,
                            guild_id,
                            position,
                            permission_overrides,
                            name,
                        } => CachedChannel::Category {
                            id: id.clone(),
                            guild_id: guild_id.clone(),
                            position: position.clone(),
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
                            id: id.clone(),
                            guild_id: guild_id.clone(),
                            position: position.clone(),
                            permission_overrides: permission_overrides.clone(),
                            name: name.clone(),
                            parent_id: parent_id.clone(),
                        },
                        CachedChannel::StoreChannel {
                            id,
                            guild_id,
                            position,
                            name,
                            parent_id,
                            permission_overrides,
                        } => CachedChannel::StoreChannel {
                            id: id.clone(),
                            guild_id: guild_id.clone(),
                            position: position.clone(),
                            name: name.clone(),
                            parent_id: parent_id.clone(),
                            permission_overrides: permission_overrides.clone(),
                        },
                    });
                }

                csg
            };
            let serialized = serde_json::to_string(&data).unwrap();
            connection
                .set_and_expire_seconds(format!("cb_guild_{}", data.id.0), serialized, 180)
                .await;
            connection
                .rpush(
                    "cb_cluster_{}_guild_list",
                    self.cluster_id,
                    data.id.0.to_string(),
                )
                .await;
        }
    }

    async fn _prepare_cold_resume_user(
        &self,
        redis_pool: &ConnectionPool,
        todo: Vec<UserId>,
        index: usize,
    ) {
        debug!(
            "Guild dumper {} started freezing {} users",
            index,
            todo.len()
        );
        let mut connection = redis_pool.get().await;
        let mut chunk = Vec::with_capacity(todo.len());
        for key in todo {
            let user = self.users.remove_take(&key).unwrap();
            chunk.push(CachedUser {
                id: user.id.clone(),
                username: user.username.clone(),
                discriminator: user.discriminator.clone(),
                avatar: user.avatar.clone(),
                bot_user: user.bot_user,
                system_user: user.system_user,
                public_flags: user.public_flags.clone(),
            });
        }
        let serialized = serde_json::to_string(&chunk).unwrap();
        connection
            .set_and_expire_seconds(
                format!("cb_cluster_{}_user_chunk_{}", self.cluster_id, index),
                serialized,
                180,
            )
            .await;
    }
}

mod structs;

pub use structs::*;

use crate::utils::Error;
use chrono::format::Numeric::Ordinal;
use darkredis::ConnectionPool;
use std::any::Any;
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use twilight::gateway::Event;
use twilight::model::channel::Channel::Guild;
use twilight::model::channel::ChannelType;
use twilight::model::channel::GuildChannel;
use twilight::model::user::User;
