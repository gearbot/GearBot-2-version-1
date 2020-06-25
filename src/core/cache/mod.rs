use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use darkredis::ConnectionPool;
use dashmap::{DashMap, ElementGuard};
use futures::future;
use log::{debug, info, trace};
use twilight::gateway::Event;
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};
use twilight::model::user::User;

pub use channel::CachedChannel;
pub use emoji::CachedEmoji;
pub use guild::{CachedGuild, ColdStorageGuild};
pub use member::{CachedMember, ColdStorageMember};
pub use role::CachedRole;
pub use user::CachedUser;

use crate::core::BotStats;
use crate::utils::Error;
use crate::{gearbot_error, gearbot_important, gearbot_info, gearbot_warn};
use std::borrow::Borrow;
use twilight::model::channel::{Channel, GuildChannel, PrivateChannel};
use twilight::model::gateway::payload::ChannelDelete;

pub struct Cache {
    //cluster info
    cluster_id: u64,

    //cache
    guilds: DashMap<GuildId, Arc<CachedGuild>>,
    guild_channels: DashMap<ChannelId, Arc<CachedChannel>>,
    private_channels: DashMap<ChannelId, Arc<CachedChannel>>,
    dm_channels_by_user: DashMap<UserId, Arc<CachedChannel>>,
    users: DashMap<UserId, Arc<CachedUser>>,
    emoji: DashMap<EmojiId, Arc<CachedEmoji>>,
    //is this even possible to get accurate across multiple clusters?
    filling: AtomicBool,

    unavailable_guilds: RwLock<Vec<GuildId>>,
    expected: RwLock<Vec<GuildId>>,

    stats: Arc<BotStats>,
}

impl Cache {
    pub fn new(cluster_id: u64, stats: Arc<BotStats>) -> Self {
        Cache {
            cluster_id,
            guilds: DashMap::new(),
            guild_channels: DashMap::new(),
            private_channels: DashMap::new(),
            dm_channels_by_user: DashMap::new(),
            users: DashMap::new(),
            emoji: DashMap::new(),
            filling: AtomicBool::new(true),
            unavailable_guilds: RwLock::new(vec![]),
            expected: RwLock::new(vec![]),
            stats,
        }
    }

    pub fn reset(&self) {
        self.guilds.clear();
        self.guild_channels.clear();
        self.users.clear();
        self.emoji.clear();
        self.filling.store(true, Ordering::SeqCst);
        self.private_channels.clear();
    }

    pub fn update(&self, event: &Event) {
        match event {
            Event::GuildCreate(e) => {
                trace!("Received guild create event for {} ({})", e.name, e.id);
                let guild = CachedGuild::from(e.0.clone());

                for channel in &guild.channels {
                    self.guild_channels.insert(channel.get_id(), channel.value().clone());
                }
                self.stats.channel_count.add(guild.channels.len() as i64);

                for emoji in &guild.emoji {
                    self.emoji.insert(emoji.id, emoji.clone());
                }
                self.stats.emoji_count.add(guild.emoji.len() as i64);

                self.stats.role_count.add(guild.roles.len() as i64);

                //we usually don't need this mutable but acquire a write lock regardless to prevent potential deadlocks
                let mut list = self.unavailable_guilds.write().unwrap();
                match list.iter().position(|id| id.0 == guild.id.0) {
                    Some(index) => {
                        list.remove(index);
                        gearbot_info!("Guild {}, ``{}`` is available again!", guild.name, guild.id);
                    }
                    None => {}
                }
                self.guilds.insert(e.id, Arc::new(guild));
                self.stats.guild_counts.partial.inc();
            }
            Event::GuildUpdate(update) => {
                trace!("Receive guild update for {} ({})", update.name, update.id);
                debug!("{:?}", update);

                match self.get_guild(update.id) {
                    Some(old_guild) => {
                        let guild = old_guild.update(&update.0);
                        self.stats.role_count.sub(old_guild.roles.len() as i64);
                        self.stats.role_count.add(guild.roles.len() as i64);
                    }
                    None => {
                        gearbot_warn!(
                            "Got a guild update for {} (``{}``) but the guild was not found in cache!",
                            update.name,
                            update.id
                        );
                    }
                }
            }
            Event::GuildEmojisUpdate(event) => {}
            Event::GuildDelete(guild) => match self.get_guild(guild.id) {
                Some(cached_guild) => {
                    if !cached_guild.complete.load(Ordering::SeqCst) {
                        self.stats.guild_counts.partial.dec();
                    } else {
                        self.stats.guild_counts.loaded.dec();
                    }

                    if guild.unavailable {
                        self.guild_unavailable(&cached_guild);
                    }
                    self.nuke_guild_cache(&cached_guild)
                }
                None => {}
            },
            Event::MemberChunk(chunk) => {
                trace!(
                    "Recieved member chunk {}/{} (nonce: {:?}) for guild {}",
                    chunk.chunk_index + 1,
                    chunk.chunk_count,
                    chunk.nonce,
                    chunk.guild_id
                );
                match self.get_guild(chunk.guild_id) {
                    Some(guild) => {
                        for (user_id, member) in chunk.members.clone() {
                            self.get_or_insert_user(&member.user);
                            let member = Arc::new(CachedMember::from_member(&member, self));
                            member.user.mutual_servers.fetch_add(1, Ordering::SeqCst);
                            guild.members.insert(user_id, member);
                        }
                        self.stats.user_counts.total.add(chunk.members.len() as i64);
                        if (chunk.chunk_count - 1) == chunk.chunk_index {
                            debug!(
                                "Finished processing all chunks for {} ({}). {:?} guilds to go!",
                                guild.name,
                                guild.id.0,
                                self.stats.guild_counts.partial.get()
                            );
                            guild.complete.store(true, Ordering::SeqCst);
                            self.stats.guild_counts.partial.dec();
                            self.stats.guild_counts.loaded.inc();
                            // if we where at 1 we are now at 0
                            if self.stats.guild_counts.partial.get() == 0
                                && self.filling.fetch_and(true, Ordering::Relaxed)
                            {
                                gearbot_important!("Initial cache filling completed for cluster {}!", self.cluster_id);
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
                //todo: add more details
                trace!("Received channel create event for channel a channel");
                match &event.0 {
                    Channel::Group(group) => {} //we do not care about groups in the slightest
                    Channel::Guild(guild_channel) => {
                        let guild_id = match guild_channel {
                            GuildChannel::Category(category) => category.guild_id,
                            GuildChannel::Text(text) => text.guild_id,
                            GuildChannel::Voice(voice) => voice.guild_id,
                        };
                        match guild_id {
                            Some(guild_id) => {
                                let channel = CachedChannel::from_guild_channel(guild_channel, guild_id);
                                match self.get_guild(guild_id) {
                                    Some(guild) => {
                                        let arced = Arc::new(channel);
                                        guild.channels.insert(arced.get_id(), arced.clone());
                                        self.guild_channels.insert(arced.get_id(), arced);
                                        self.stats.channel_count.inc();
                                    }
                                    None => gearbot_error!(
                                        "Channel create received for #{} **``{}``** in guild **``{}``** but this guild does not exist in cache!",
                                        channel.get_name(),
                                        channel.get_id(),
                                        guild_id
                                    ),
                                }
                            }
                            None => gearbot_warn!(
                                "We got a channel create event for a guild type channel without guild id!"
                            ),
                        }
                    }
                    Channel::Private(private_channel) => {
                        self.insert_private_channel(private_channel);
                    }
                };
            }
            Event::ChannelUpdate(channel) => {
                match &channel.0 {
                    Channel::Group(_) => {} //get out of here!
                    Channel::Guild(guild_channel) => {
                        let guild_id = match guild_channel {
                            GuildChannel::Category(cateogry) => cateogry.guild_id,
                            GuildChannel::Text(text) => text.guild_id,
                            GuildChannel::Voice(voice) => voice.guild_id,
                        };
                        match guild_id {
                            Some(guild_id) => match self.get_guild(guild_id) {
                                Some(guild) => {
                                    let channel = CachedChannel::from_guild_channel(guild_channel, guild.id);
                                    let arced = Arc::new(channel);
                                    guild.channels.insert(arced.get_id(), arced.clone());
                                    self.guild_channels.insert(arced.get_id(), arced);
                                }
                                None => gearbot_warn!(
                                    "Got a channel update for guild ``{}`` but we do not have this guild cached!",
                                    guild_id
                                ),
                            },
                            None => gearbot_warn!(
                                "Got a channel update for  of a guild type channel but it did not have a guild id!"
                            ),
                        }
                    }
                    Channel::Private(private) => {
                        self.insert_private_channel(private);
                    }
                }
            }
            Event::ChannelDelete(channel) => {
                //todo: add more info
                trace!("Got a channel delete event for a channel");

                match &channel.0 {
                    Channel::Group(_) => {} //nope, still don't care
                    Channel::Guild(guild_channel) => {
                        let (guild_id, channel_id) = match guild_channel {
                            GuildChannel::Text(text) => (text.guild_id, text.id),
                            GuildChannel::Voice(voice) => (voice.guild_id, voice.id),
                            GuildChannel::Category(category) => (category.guild_id, category.id),
                        };
                        match guild_id {
                            Some(guild_id) => match self.get_guild(guild_id) {
                                Some(guild) => {
                                    guild.channels.remove(&channel_id);
                                    self.stats.channel_count.dec();
                                }
                                None => {
                                    gearbot_warn!("Got a channel delete for channel ``{}`` event for guild ``{}`` but we do not have this guild in the cache", channel_id, guild_id);
                                }
                            },
                            None => {
                                gearbot_warn!("Got a channel delete for channel ``{}`` that is supposed to belong to a guild but does not have a guild id attached", channel_id);
                            }
                        }
                    }
                    //Do these even ever get deleted?
                    Channel::Private(channel) => {
                        self.private_channels.remove(&channel.id);
                        if channel.recipients.len() == 1 {
                            self.dm_channels_by_user.remove(&channel.recipients[0].id);
                        }
                    }
                }
            }

            Event::MemberAdd(event) => {
                //TODO: remove unwrap once we update twilight
                match self.get_guild(event.guild_id) {
                    Some(guild) => {
                        guild
                            .members
                            .insert(event.user.id, Arc::new(CachedMember::from_member(&event.0, &self)));
                        guild.member_count.fetch_add(1, Ordering::Relaxed);
                        self.stats.user_counts.total.inc();
                    }
                    None => gearbot_warn!(
                        "Got a member add event for guild {} before guild create",
                        event.guild_id
                    ),
                }
            }

            Event::MemberUpdate(event) => {
                //TODO: remove unwrap once we update twilight
                match self.get_guild(event.guild_id) {
                    Some(guild) => {
                        match self.get_user(event.user.id) {
                            Some(user) => {
                                if !user.is_same_as(&event.user) {
                                    //just update the global cache if it's different, we will receive an event for all mutual servers if the inner user changed
                                    self.users
                                        .insert(event.user.id, Arc::new(CachedUser::from_user(&event.user)));
                                }
                            }
                            None => gearbot_warn!("Received a member update with an uncached inner user!"),
                        }
                        match guild.members.get(&event.user.id) {
                            Some(member) => {
                                guild
                                    .members
                                    .insert(member.user.id, Arc::new(member.update(&*event, &self)));
                            }
                            None => gearbot_warn!(
                                "Received a member update for an unknown member in guild {}",
                                event.guild_id
                            ),
                        }
                    }
                    None => {
                        gearbot_warn!("Received a member update for an uncached guild: {}", event.guild_id);
                    }
                }
            }

            Event::MemberRemove(event) => match self.get_guild(event.guild_id) {
                Some(guild) => match guild.members.remove_take(&event.user.id) {
                    Some(member) => {
                        let servers = member.user.mutual_servers.fetch_sub(1, Ordering::SeqCst);
                        if servers == 1 {
                            self.users.remove(&member.user.id);
                            self.stats.user_counts.unique.dec();
                        }
                        self.stats.user_counts.total.dec();
                    }
                    None => gearbot_warn!("Received a member remove event for a member that is not in that guild"),
                },
                None => gearbot_warn!(
                    "Received a member remove for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            Event::RoleCreate(event) => match self.get_guild(event.guild_id) {
                Some(guild) => {
                    guild
                        .roles
                        .insert(event.role.id, Arc::new(CachedRole::from_role(&event.role)));
                    self.stats.role_count.inc();
                }
                None => gearbot_warn!(
                    "Received a role create event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            Event::RoleUpdate(event) => match self.get_guild(event.guild_id) {
                Some(guild) => {
                    guild
                        .roles
                        .insert(event.role.id, Arc::new(CachedRole::from_role(&event.role)));
                }
                None => gearbot_warn!(
                    "Received a role update event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            Event::RoleDelete(event) => match self.get_guild(event.guild_id) {
                Some(guild) => {
                    guild.roles.remove(&event.role_id);
                    self.stats.role_count.dec();
                }
                None => gearbot_warn!(
                    "Received a role delete event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            _ => {}
        }
    }

    fn guild_unavailable(&self, guild: &Arc<CachedGuild>) {
        gearbot_warn!(
            "Guild \"{}\", ``{}`` became unavailable due to an outage",
            guild.name,
            guild.id
        );
        self.stats.guild_counts.outage.inc();
        let mut list = self.unavailable_guilds.write().unwrap();
        list.push(guild.id);
    }

    fn nuke_guild_cache(&self, guild: &Arc<CachedGuild>) {
        for channel in &guild.channels {
            self.guild_channels.remove(channel.key());
        }
        self.stats.channel_count.sub(guild.channels.len() as i64);

        for member in &guild.members {
            let remaining = member.user.mutual_servers.fetch_sub(1, Ordering::SeqCst);
            if remaining == 1 {
                self.users.remove(&member.user.id);
                self.stats.user_counts.unique.dec();
            }
        }
        self.stats.user_counts.total.sub(guild.members.len() as i64);

        for emoji in &guild.emoji {
            self.emoji.remove(&emoji.id);
        }
        self.stats.emoji_count.sub(guild.emoji.len() as i64);
        self.stats.role_count.sub(guild.roles.len() as i64);
    }

    pub fn insert_private_channel(&self, private_channel: &PrivateChannel) -> Arc<CachedChannel> {
        let channel = CachedChannel::from_private(private_channel, self);
        let arced = Arc::new(channel);
        match arced.as_ref() {
            CachedChannel::DM { receiver, .. } => {
                self.dm_channels_by_user.insert(receiver.id, arced.clone());
            }
            _ => {}
        };
        self.private_channels.insert(arced.get_id(), arced.clone());
        arced
    }

    pub fn get_or_insert_user(&self, user: &User) -> Arc<CachedUser> {
        match self.get_user(user.id) {
            Some(user) => user,
            None => {
                let arc = Arc::new(CachedUser::from_user(user));
                self.users.insert(arc.id, arc.clone());
                self.stats.user_counts.unique.inc();
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
            None => match self.private_channels.get(&channel_id) {
                Some(guard) => Some(guard.value().clone()),
                None => None,
            },
        }
    }

    pub fn get_dm_channel_for(&self, user_id: UserId) -> Option<Arc<CachedChannel>> {
        match self.dm_channels_by_user.get(&user_id) {
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
        //we do not want to drag along DM channels, we get guild creates for them when they send a message anyways
        self.private_channels.clear();

        //let's go to hyperspeed
        let mut tasks = vec![];
        let mut user_tasks = vec![];

        //but not yet, collect their work first before they start sabotaging each other again >.>
        let mut work_orders: Vec<Vec<GuildId>> = vec![];

        let mut count = 0;
        let mut list = vec![];
        for guard in self.guilds.iter() {
            count += guard.members.len() + guard.channels.len() + guard.emoji.len() + guard.roles.len();
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
        debug!("Freezing {:?} guilds", self.stats.guild_counts.loaded.get());
        for i in 0..work_orders.len() {
            tasks.push(self._prepare_cold_resume_guild(redis_pool, work_orders[i].clone(), i));
        }
        let guild_chunks = tasks.len();

        future::join_all(tasks).await;

        count = 0;
        let user_chunks = (self.users.len() / 100000 + 1) as usize;
        let mut user_work_orders: Vec<Vec<UserId>> = vec![vec![]; user_chunks];
        for guard in self.users.iter() {
            user_work_orders[count % user_chunks].push(guard.key().clone());
            count += 1;
        }
        debug!("Freezing {:?} users", self.users.len());
        for i in 0..user_chunks {
            user_tasks.push(self._prepare_cold_resume_user(redis_pool, user_work_orders[i].clone(), i));
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
        debug!("Guild dumper {} started freezing {} guilds", index, todo.len());
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
                mutual_servers: AtomicU64::new(0),
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
                    return Err(Error::CacheDefrostError(format!("Failed to defrost users: {}", e)));
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
                    return Err(Error::CacheDefrostError(format!("Failed to defrost guilds: {}", e)));
                }
                Ok(_) => {}
            }
        }

        self.filling.store(false, Ordering::SeqCst);
        info!(
            "Cache defrosting complete! Now holding {} users ({} unique) from {} guilds, good for a total of {} roles, {} channels and {} emoji.",
            self.stats.user_counts.total.get(),
            self.stats.user_counts.unique.get(),
            self.stats.guild_counts.loaded.get(),
            self.stats.role_count.get(),
            self.stats.channel_count.get(),
            self.stats.emoji_count.get()
        );

        Ok(())
    }

    async fn defrost_users(&self, redis_pool: &ConnectionPool, index: usize) -> Result<(), Error> {
        let key = format!("cb_cluster_{}_user_chunk_{}", self.cluster_id, index);
        let mut connection = redis_pool.get().await;
        let mut users: Vec<CachedUser> =
            serde_json::from_str(&*String::from_utf8(connection.get(&key).await?.unwrap()).unwrap())?;
        connection.del(key).await?;
        debug!("Worker {} found {} users to defrost", index, users.len());
        for user in users.drain(..) {
            self.users.insert(user.id, Arc::new(user));
            self.stats.user_counts.unique.inc();
        }

        Ok(())
    }

    async fn defrost_guilds(&self, redis_pool: &ConnectionPool, index: usize) -> Result<(), Error> {
        let key = format!("cb_cluster_{}_guild_chunk_{}", self.cluster_id, index);
        let mut connection = redis_pool.get().await;
        let mut guilds: Vec<ColdStorageGuild> =
            serde_json::from_str(&*String::from_utf8(connection.get(&key).await?.unwrap()).unwrap())?;
        connection.del(key).await?;
        debug!("Worker {} found {} guilds to defrost", index, guilds.len());
        for cold_guild in guilds.drain(..) {
            let guild = CachedGuild::defrost(&self, cold_guild);

            for channel in &guild.channels {
                self.guild_channels.insert(channel.get_id(), channel.value().clone());
            }
            self.stats.channel_count.add(guild.channels.len() as i64);

            for emoji in &guild.emoji {
                self.emoji.insert(emoji.id, emoji.clone());
            }
            self.stats.emoji_count.add(guild.emoji.len() as i64);

            self.stats.user_counts.total.add(guild.members.len() as i64);

            self.guilds.insert(guild.id, Arc::new(guild));
            self.stats.guild_counts.loaded.inc();
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
        mutual_servers: AtomicU64::new(old.mutual_servers.load(Ordering::SeqCst)),
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
