use crate::{gearbot_error, gearbot_important, gearbot_info};
use dashmap::DashMap;
use futures::future;
use log::{debug, error, info};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};

pub struct Cache {
    //cluster info
    cluster_id: u64,

    //cache
    guilds: DashMap<GuildId, Arc<CachedGuild>>,
    guild_channels: DashMap<ChannelId, Arc<CachedChannel>>,
    users: DashMap<UserId, Arc<CachedUser>>,
    emoji: DashMap<EmojiId, Arc<CachedEmoji>>,
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

    pub fn reset(&self) {
        self.guilds.clear();
        self.guild_channels.clear();
        self.users.clear();
        self.emoji.clear();
        self.guild_count.store(0, Ordering::SeqCst);
        self.unique_users.store(0, Ordering::SeqCst);
        self.total_users.store(0, Ordering::SeqCst);
        self.partial_guilds.store(0, Ordering::SeqCst);
        self.role_count.store(0, Ordering::SeqCst);
        self.channel_count.store(0, Ordering::SeqCst);
        self.emoji_count.store(0, Ordering::SeqCst);
        self.filling.store(true, Ordering::SeqCst);
    }

    pub fn update(&self, event: &Event) {
        match event {
            Event::GuildCreate(e) => {
                let guild = CachedGuild::from(e.0.clone());

                for channel in &guild.channels {
                    self.guild_channels
                        .insert(channel.get_id(), channel.value().clone());
                }
                self.channel_count
                    .fetch_add(guild.channels.len() as u64, Ordering::Relaxed);

                for emoji in &guild.emoji {
                    self.emoji.insert(emoji.id, emoji.clone());
                }
                self.emoji_count
                    .fetch_add(guild.emoji.len() as u64, Ordering::Relaxed);

                self.guilds.insert(e.id, Arc::new(guild));
                self.guild_count.fetch_add(1, Ordering::Relaxed);
                self.partial_guilds.fetch_add(1, Ordering::Relaxed);
            }

            Event::MemberChunk(chunk) => {
                match self.get_guild(chunk.guild_id) {
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
                                guild.complete.store(true, Ordering::SeqCst);
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
        match self.get_user(user.id) {
            Some(user) => user,
            None => {
                let arc = Arc::new(CachedUser::from(user));
                self.users.insert(arc.id, arc.clone());
                self.unique_users.fetch_add(1, Ordering::Relaxed);
                arc
            }
        }
    }

    pub fn get_guild(&self, guild_id: GuildId) -> Option<Arc<CachedGuild>> {
        match self.guilds.get(&guild_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<CachedChannel>> {
        match self.guild_channels.get(&channel_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_user(&self, user_id: UserId) -> Option<Arc<CachedUser>> {
        match self.users.get(&user_id) {
            Some(guard) => Some(guard.value().clone()),
            None => None,
        }
    }

    pub fn get_member(&self, guild_id: GuildId, user_id: UserId) -> Option<Arc<CachedMember>> {
        match self.guilds.get(&guild_id) {
            Some(guard) => match guard.value().members.get(&user_id) {
                Some(guard) => Some(guard.value().clone()),
                None => None,
            },
            None => None,
        }
    }

    pub async fn prepare_cold_resume(&self, redis_pool: &ConnectionPool) -> (usize, usize) {
        //clear global caches so arcs can be cleaned up
        self.guild_channels.clear();
        //let's go to hyperspeed
        let mut tasks = vec![];
        let mut user_tasks = vec![];

        //but not yet, collect their work first before they start sabotaging each other again >.>
        let mut work_orders: Vec<Vec<GuildId>> = vec![];

        let mut count = 0;
        let mut list = vec![];
        for guard in self.guilds.iter() {
            count +=
                guard.members.len() + guard.channels.len() + guard.emoji.len() + guard.roles.len();
            list.push(guard.key().clone());
            if count > 100000 {
                work_orders.push(list);
                list = vec![];
                count = 0;
            }
        }
        if list.len() > 0 {
            work_orders.push(list)
        }
        debug!("Freezing {:?} guilds", self.guild_count);
        for i in 0..work_orders.len() {
            tasks.push(self._prepare_cold_resume_guild(redis_pool, work_orders[i].clone(), i));
        }
        let guild_chunks = tasks.len();

        future::join_all(tasks).await;

        count = 0;
        let user_chunks = (self.unique_users.load(Ordering::Relaxed) / 100000 + 1) as usize;
        let mut user_work_orders: Vec<Vec<UserId>> = vec![vec![]; user_chunks];
        for guard in self.users.iter() {
            user_work_orders[count % user_chunks].push(guard.key().clone());
            count += 1;
        }
        debug!("Freezing {:?} users", self.unique_users);
        for i in 0..user_chunks {
            user_tasks.push(self._prepare_cold_resume_user(
                redis_pool,
                user_work_orders[i].clone(),
                i,
            ));
        }

        future::join_all(user_tasks).await;
        self.users.clear();
        (guild_chunks, user_chunks)
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
        let mut to_dump = Vec::with_capacity(todo.len());
        for key in todo {
            let g = self.guilds.remove_take(&key).unwrap();

            to_dump.push(ColdStorageGuild::from(g));
        }
        let serialized = serde_json::to_string(&to_dump).unwrap();
        connection
            .set_and_expire_seconds(
                format!("cb_cluster_{}_guild_chunk_{}", self.cluster_id, index),
                serialized,
                180,
            )
            .await;
    }

    async fn _prepare_cold_resume_user(
        &self,
        redis_pool: &ConnectionPool,
        todo: Vec<UserId>,
        index: usize,
    ) {
        debug!("Worker {} freezing {} users", index, todo.len());
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

    pub async fn restore_cold_resume(
        &self,
        redis_pool: &ConnectionPool,
        guild_chunks: usize,
        user_chunks: usize,
    ) -> Result<(), Error> {
        let mut user_defrosters = Vec::with_capacity(user_chunks);

        for i in 0..user_chunks {
            user_defrosters.push(self.defrost_users(redis_pool, i));
        }

        for result in future::join_all(user_defrosters).await {
            match result {
                Err(e) => {
                    return Err(Error::CacheDefrostError(format!(
                        "Failed to defrost users: {}",
                        e
                    )))
                }
                Ok(_) => {}
            }
        }

        let mut guild_defrosters = Vec::with_capacity(guild_chunks);

        for i in 0..guild_chunks {
            guild_defrosters.push(self.defrost_guilds(redis_pool, i));
        }

        for result in future::join_all(guild_defrosters).await {
            match result {
                Err(e) => {
                    return Err(Error::CacheDefrostError(format!(
                        "Failed to defrost guilds: {}",
                        e
                    )))
                }
                Ok(_) => {}
            }
        }
        self.filling.store(false, Ordering::SeqCst);
        info!("Cache defrosting complete! Now holding {} users ({} unique) from {} guilds, good for a total of {} roles, {} channels and {} emoji.", self.total_users.load(Ordering::Relaxed), self.unique_users.load(Ordering::Relaxed), self.guild_count.load(Ordering::Relaxed), self.role_count.load(Ordering::Relaxed), self.channel_count.load(Ordering::Relaxed), self.emoji_count.load(Ordering::Relaxed));

        Ok(())
    }

    async fn defrost_users(&self, redis_pool: &ConnectionPool, index: usize) -> Result<(), Error> {
        let key = format!("cb_cluster_{}_user_chunk_{}", self.cluster_id, index);
        let mut connection = redis_pool.get().await;
        let mut users: Vec<CachedUser> = serde_json::from_str(
            &*String::from_utf8(connection.get(&key).await?.unwrap()).unwrap(),
        )?;
        connection.del(key).await?;
        debug!("Worker {} found {} users to defrost", index, users.len());
        for user in users.drain(..) {
            self.users.insert(user.id, Arc::new(user));
            self.unique_users.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    async fn defrost_guilds(&self, redis_pool: &ConnectionPool, index: usize) -> Result<(), Error> {
        let key = format!("cb_cluster_{}_guild_chunk_{}", self.cluster_id, index);
        let mut connection = redis_pool.get().await;
        let mut guilds: Vec<ColdStorageGuild> = serde_json::from_str(
            &*String::from_utf8(connection.get(&key).await?.unwrap()).unwrap(),
        )?;
        connection.del(key).await?;
        debug!("Worker {} found {} guilds to defrost", index, guilds.len());
        for cold_guild in guilds.drain(..) {
            let guild = CachedGuild::defrost(&self, cold_guild);

            for channel in &guild.channels {
                self.guild_channels
                    .insert(channel.get_id(), channel.value().clone());
            }
            self.channel_count
                .fetch_add(guild.channels.len() as u64, Ordering::Relaxed);

            for emoji in &guild.emoji {
                self.emoji.insert(emoji.id, emoji.clone());
            }
            self.emoji_count
                .fetch_add(guild.emoji.len() as u64, Ordering::Relaxed);

            self.total_users
                .fetch_add(guild.members.len() as u64, Ordering::Relaxed);

            self.guilds.insert(guild.id, Arc::new(guild));
            self.guild_count.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }
}

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

mod guild;
pub use guild::{CachedGuild, ColdStorageGuild};
mod role;
pub use role::CachedRole;
mod emoji;
pub use emoji::CachedEmoji;
mod member;
pub use member::{CachedMember, ColdStorageMember};
mod channel;
pub use channel::CachedChannel;
mod user;
pub use user::CachedUser;

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

fn is_true(t: &bool) -> bool {
    !t
}

fn get_true() -> bool {
    true
}
