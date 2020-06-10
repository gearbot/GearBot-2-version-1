use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use darkredis::ConnectionPool;
use dashmap::DashMap;
use futures::future;
use log::{debug, info};
use twilight::gateway::Event;
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};
use twilight::model::user::User;

pub use channel::CachedChannel;
pub use emoji::CachedEmoji;
pub use guild::{CachedGuild, ColdStorageGuild};
pub use member::{CachedMember, ColdStorageMember};
pub use role::CachedRole;
pub use user::CachedUser;

use crate::utils::Error;
use crate::{gearbot_error, gearbot_important, gearbot_warn};

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
                            self.get_or_insert_user(member.user.clone());
                            let member = Arc::new(CachedMember::from_member(member, self));
                            guild.members.insert(user_id, member);
                        }
                        if (chunk.chunk_count - 1) == chunk.chunk_index {
                            debug!(
                                "Finished processing all chunks for {} ({}). {:?} guilds to go!",
                                guild.name, guild.id.0, self.partial_guilds
                            );
                            guild.complete.store(true, Ordering::SeqCst);
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
            Event::ChannelCreate(event) => {
                // match event.0 {
                //     Channel::Group {
                //
                //     }
                // }
                //
                // let guild_id = match event.0 {
                //
                // }
                // let channel = CachedChannel::from(event.0.clone(), guild_id);
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

    /// we get member updates for all
    pub fn update_user(&self, new: Arc<CachedUser>) {
        match self.get_user(new.id) {
            Some(old) => {
                let updated = update_user_with_user(old, new);
                let user = Arc::new(updated);
                for guard in &self.guilds {
                    if let Some(member) = guard.members.get(&user.id) {
                        guard
                            .members
                            .insert(user.id, Arc::new(member.replace_user(user.clone())));
                    }
                }
                self.users.insert(user.id, user);
            }
            None => {
                gearbot_warn!(
                    "Trying to update user {}#{} (``{}``) but they where not found in the cache!",
                    new.username,
                    new.discriminator,
                    new.id.0
                );
            }
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
    ) -> Result<(), Error> {
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
            .await?;
        Ok(())
    }

    async fn _prepare_cold_resume_user(
        &self,
        redis_pool: &ConnectionPool,
        todo: Vec<UserId>,
        index: usize,
    ) -> Result<(), Error> {
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
            .await?;

        Ok(())
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
                    )));
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
                    )));
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

mod guild;

mod role;

mod emoji;

mod member;

mod channel;

mod user;

fn update_user_with_user(old: Arc<CachedUser>, new: Arc<CachedUser>) -> CachedUser {
    let public_flags = match new.public_flags {
        Some(flags) => Some(flags),
        None => old.public_flags,
    };
    CachedUser {
        id: old.id,
        username: new.username.clone(),
        discriminator: new.discriminator.clone(),
        avatar: new.avatar.clone(),
        bot_user: new.bot_user,
        system_user: new.system_user,
        public_flags,
    }
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}

fn is_true(t: &bool) -> bool {
    !t
}

fn get_true() -> bool {
    true
}
