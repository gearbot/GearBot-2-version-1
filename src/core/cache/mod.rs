use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use darkredis::ConnectionPool;
use futures_util::future;
use log::{debug, info, trace, warn};
use twilight::gateway::Event;
use twilight::model::id::{ChannelId, EmojiId, GuildId, UserId};
use twilight::model::user::User;

pub use channel::CachedChannel;
pub use emoji::CachedEmoji;
pub use guild::{CachedGuild, ColdStorageGuild};
pub use member::CachedMember;
pub use role::CachedRole;
pub use user::CachedUser;

use crate::core::context::bot::ShardState;
use crate::core::{BotContext, BotStats};
use crate::utils::Error;
use crate::{gearbot_error, gearbot_important, gearbot_info, gearbot_warn};
use std::collections::HashMap;
use twilight::model::channel::{Channel, GuildChannel, PrivateChannel};
use twilight::model::gateway::payload::RequestGuildMembers;
use twilight::model::gateway::presence::{ActivityType, Status};

pub struct Cache {
    //cluster info
    cluster_id: u64,

    //cache
    pub guilds: RwLock<HashMap<GuildId, Arc<CachedGuild>>>,
    pub guild_channels: RwLock<HashMap<ChannelId, Arc<CachedChannel>>>,
    pub private_channels: RwLock<HashMap<ChannelId, Arc<CachedChannel>>>,
    pub dm_channels_by_user: RwLock<HashMap<UserId, Arc<CachedChannel>>>,
    pub users: RwLock<HashMap<UserId, Arc<CachedUser>>>,
    pub emoji: RwLock<HashMap<EmojiId, Arc<CachedEmoji>>>,
    //is this even possible to get accurate across multiple clusters?
    pub filling: AtomicBool,

    pub unavailable_guilds: RwLock<Vec<GuildId>>,
    pub expected: RwLock<Vec<GuildId>>,

    pub stats: Arc<BotStats>,
    pub missing_per_shard: RwLock<HashMap<u64, AtomicU64>>,
}

impl Cache {
    pub fn new(cluster_id: u64, stats: Arc<BotStats>) -> Self {
        Cache {
            cluster_id,
            guilds: RwLock::new(HashMap::new()),
            guild_channels: RwLock::new(HashMap::new()),
            private_channels: RwLock::new(HashMap::new()),
            dm_channels_by_user: RwLock::new(HashMap::new()),
            users: RwLock::new(HashMap::new()),
            emoji: RwLock::new(HashMap::new()),
            filling: AtomicBool::new(true),
            unavailable_guilds: RwLock::new(vec![]),
            expected: RwLock::new(vec![]),
            stats,
            missing_per_shard: RwLock::new(HashMap::new()),
        }
    }

    pub fn reset(&self) {
        self.guilds.write().expect("Global guilds cache got poisoned!").clear();
        self.guild_channels
            .write()
            .expect("Global guild channels cache got poisoned!")
            .clear();
        self.users.write().expect("Global users cache got poisoned!").clear();
        self.emoji.write().expect("Global emoji cache got poisoned").clear();
        self.filling.store(true, Ordering::SeqCst);
        self.private_channels
            .write()
            .expect("Global private channel cache got poisoned!")
            .clear();
    }

    pub fn update(&self, shard_id: u64, event: &Event, ctx: Arc<BotContext>) {
        match event {
            Event::Ready(ready) => {
                self.missing_per_shard
                    .write()
                    .expect("Global shard state tracking got poisoned!")
                    .insert(shard_id, AtomicU64::new(ready.guilds.len() as u64));
                // just in case somehow got here without getting any re-identifying event
                // shouldn't happen but memory leaks are very bad
                for gid in ready.guilds.keys() {
                    if let Some(guild) = self.get_guild(gid) {
                        self.nuke_guild_cache(&guild)
                    }
                }
            }
            Event::GuildCreate(e) => {
                trace!("Received guild create event for {} ({})", e.name, e.id);
                if let Some(cached_guild) = self
                    .guilds
                    .read()
                    .expect("Global guilds cache got poisoned!")
                    .get(&e.id)
                {
                    self.nuke_guild_cache(cached_guild)
                }
                let guild = CachedGuild::from(e.0.clone());

                {
                    let mut guild_channels = self
                        .guild_channels
                        .write()
                        .expect("Global guild channels cache got poisoned!");
                    let gc = guild.channels.read().expect("Guild inner channel cache got poisoned!");
                    for channel in gc.values() {
                        guild_channels.insert(channel.get_id(), channel.clone());
                    }
                    self.stats.channel_count.add(gc.len() as i64);
                }

                {
                    let mut emoji_cache = self.emoji.write().expect("Global emoji cache got poisoned!");
                    for emoji in &guild.emoji {
                        emoji_cache.insert(emoji.id, emoji.clone());
                    }
                    self.stats.emoji_count.add(guild.emoji.len() as i64);
                }

                self.stats
                    .role_count
                    .add(guild.roles.read().expect("Guild inner roles cache got poisoned!").len() as i64);

                //we usually don't need this mutable but acquire a write lock regardless to prevent potential deadlocks
                let mut list = self.unavailable_guilds.write().unwrap();
                if let Some(index) = list.iter().position(|id| id.0 == guild.id.0) {
                    list.remove(index);
                    gearbot_info!("Guild {}, ``{}`` is available again!", guild.name, guild.id);
                }

                self.guilds
                    .write()
                    .expect("Global guild cache got poisoned!")
                    .insert(e.id, Arc::new(guild));
                self.stats.guild_counts.partial.inc();
            }
            Event::GuildUpdate(update) => {
                trace!("Receive guild update for {} ({})", update.name, update.id);

                match self.get_guild(&update.id) {
                    Some(old_guild) => {
                        let guild = old_guild.update(&update.0);
                        self.stats.role_count.sub(
                            old_guild
                                .roles
                                .read()
                                .expect("Guild inner role cache got poisoned!")
                                .len() as i64,
                        );
                        self.stats
                            .role_count
                            .add(guild.roles.read().expect("Guild inner role cache got poisoned!").len() as i64);
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
            Event::GuildEmojisUpdate(_) => {}
            Event::GuildDelete(guild) => {
                if let Some(cached_guild) = self.get_guild(&guild.id) {
                    if guild.unavailable {
                        self.guild_unavailable(&cached_guild);
                    }
                    self.nuke_guild_cache(&cached_guild)
                }
            }
            Event::MemberChunk(chunk) => {
                trace!(
                    "Received member chunk {}/{} (nonce: {:?}) for guild {}",
                    chunk.chunk_index + 1,
                    chunk.chunk_count,
                    chunk.nonce,
                    chunk.guild_id
                );
                match self.get_guild(&chunk.guild_id) {
                    Some(guild) => {
                        let mut count = 0;
                        for (user_id, member) in &chunk.members {
                            let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
                            if !members.contains_key(user_id) {
                                count += 1;
                                self.get_or_insert_user(&member.user);
                                let member = Arc::new(CachedMember::from_member(member));
                                let count = member.user(self).mutual_servers.fetch_add(1, Ordering::SeqCst) + 1;

                                trace!(
                                    "{} received for {}, they are now in {} mutuals",
                                    user_id,
                                    guild.id,
                                    count,
                                );
                                members.insert(*user_id, member);
                            }
                        }
                        self.stats.user_counts.total.add(count);
                        if (chunk.chunk_count - 1) == chunk.chunk_index && chunk.nonce.is_none() {
                            debug!(
                                "Finished processing all chunks for {} ({}). {:?} guilds to go!",
                                guild.name,
                                guild.id.0,
                                self.stats.guild_counts.partial.get()
                            );
                            guild.complete.store(true, Ordering::SeqCst);
                            let shard_missing = self
                                .missing_per_shard
                                .read()
                                .expect("Global shard state tracking got poisoned!")
                                .get(&shard_id)
                                .unwrap()
                                .fetch_sub(1, Ordering::Relaxed);
                            if shard_missing == 1 {
                                //this shard is ready
                                info!("All guilds cached for shard {}", shard_id);
                                if chunk.nonce.is_none() && self.shard_cached(shard_id) {
                                    let c = ctx.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = c
                                            .set_shard_activity(
                                                shard_id,
                                                Status::Online,
                                                ActivityType::Watching,
                                                String::from("the gears turn"),
                                            )
                                            .await
                                        {
                                            gearbot_error!(
                                                "Failed to set shard activity for shard {}: {}",
                                                shard_id,
                                                e
                                            );
                                        }
                                    });
                                }
                            }
                            self.stats.guild_counts.partial.dec();
                            self.stats.guild_counts.loaded.inc();
                            // if we where at 1 we are now at 0
                            if self.stats.guild_counts.partial.get() == 0
                                && self.filling.load(Ordering::Relaxed)
                                && ctx
                                    .shard_states
                                    .read()
                                    .expect("Shard states got poisoned")
                                    .values()
                                    .all(|state| match state {
                                        ShardState::Ready => true,
                                        _ => false,
                                    })
                            {
                                gearbot_important!("Initial cache filling completed for cluster {}!", self.cluster_id);
                                self.filling.store(false, Ordering::SeqCst);
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
                    Channel::Group(_) => {} //we do not care about groups in the slightest
                    Channel::Guild(guild_channel) => {
                        let guild_id = match guild_channel {
                            GuildChannel::Category(category) => category.guild_id,
                            GuildChannel::Text(text) => text.guild_id,
                            GuildChannel::Voice(voice) => voice.guild_id,
                        };
                        match guild_id {
                            Some(guild_id) => {
                                let channel = CachedChannel::from_guild_channel(guild_channel, guild_id);
                                match self.get_guild(&guild_id) {
                                    Some(guild) => {
                                        let arced = Arc::new(channel);
                                        guild.channels.write().expect("Guild inner channel cache got poisoned!").insert(arced.get_id(), arced.clone());
                                        self.guild_channels.write().expect("Global guild channels cache got poisoned!").insert(arced.get_id(), arced);
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
                            Some(guild_id) => match self.get_guild(&guild_id) {
                                Some(guild) => {
                                    let channel = CachedChannel::from_guild_channel(guild_channel, guild.id);
                                    let arced = Arc::new(channel);
                                    guild
                                        .channels
                                        .write()
                                        .expect("Guild inner channels cache got poisoned!")
                                        .insert(arced.get_id(), arced.clone());
                                    self.guild_channels
                                        .write()
                                        .expect("Global guild channel cache got poisoned!")
                                        .insert(arced.get_id(), arced);
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
                            Some(guild_id) => match self.get_guild(&guild_id) {
                                Some(guild) => {
                                    self.guild_channels
                                        .write()
                                        .expect("Global guild channels cache got poisoned!")
                                        .remove(&channel_id);
                                    guild
                                        .channels
                                        .write()
                                        .expect("Guild inner channels cache got poisoned!")
                                        .remove(&channel_id);
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
                        self.private_channels
                            .write()
                            .expect("Global private channels cache got poisoned!")
                            .remove(&channel.id);
                        if channel.recipients.len() == 1 {
                            self.dm_channels_by_user
                                .write()
                                .expect("Global DM channel cache got poisoned!")
                                .remove(&channel.recipients[0].id);
                        }
                    }
                }
            }

            Event::MemberAdd(event) => {
                debug!("{} joined {}", event.user.id, event.guild_id);
                match self.get_guild(&event.guild_id) {
                    Some(guild) => {
                        let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
                        let user = self.get_or_insert_user(&event.user);
                        if !members.contains_key(&event.user.id) {
                            let member = CachedMember::from_member(&event.0);
                            let count = user.mutual_servers.fetch_add(1, Ordering::SeqCst) + 1;

                            debug!("{} is now in {} mutual servers", member.user_id, count);
                            members.insert(event.user.id, Arc::new(member));
                            guild.member_count.fetch_add(1, Ordering::Relaxed);
                            self.stats.user_counts.total.inc();
                        }
                    }
                    None => gearbot_warn!(
                        "Got a member add event for guild {} before guild create",
                        event.guild_id
                    ),
                }
            }

            Event::MemberUpdate(event) => {
                trace!("{} updated in {}", event.user.id, event.guild_id);
                match ctx.cache.get_guild(&event.guild_id) {
                    Some(guild) => {
                        match ctx.cache.get_user(event.user.id) {
                            Some(user) => {
                                if !user.is_same_as(&event.user) {
                                    //just update the global cache if it's different, we will receive an event for all mutual servers if the inner user changed
                                    let new_user = Arc::new(CachedUser::from_user(&event.user));
                                    new_user
                                        .mutual_servers
                                        .store(user.mutual_servers.load(Ordering::SeqCst), Ordering::SeqCst);
                                    ctx.cache
                                        .users
                                        .write()
                                        .expect("Global user cache got poisoned!")
                                        .insert(event.user.id, new_user);
                                }
                            }
                            None => {
                                if guild.complete.load(Ordering::SeqCst) {
                                    warn!(
                                        "Received a member update with an uncached inner user: {}",
                                        event.user.id
                                    );
                                    ctx.cache.get_or_insert_user(&event.user);
                                }
                            }
                        }
                        let mut members = guild.members.write().expect("Guild inner members cache got poisoned!");
                        if members.contains_key(&event.user.id) {
                            let g = {
                                let member = members.get(&event.user.id).unwrap();
                                Arc::new(member.update(&*event))
                            };
                            members.insert(event.user.id, g);
                        } else if guild.complete.load(Ordering::SeqCst) {
                            warn!(
                                "Received a member update for an unknown member {} in guild {}",
                                event.user.id, guild.id
                            );
                            let id = event.user.id;
                            let gid = guild.id;
                            tokio::spawn(async move {
                                let data = RequestGuildMembers::new_single_user_with_nonce(
                                    gid,
                                    id,
                                    None,
                                    Some(String::from("missing_user")),
                                );
                                let _ = ctx.cluster.command(shard_id, &data).await;
                            });
                        }
                    }
                    None => {
                        gearbot_warn!("Received a member update for an uncached guild: {}", event.guild_id);
                    }
                }
            }

            Event::MemberRemove(event) => {
                debug!("{} left {}", event.user.id, event.guild_id);
                match self.get_guild(&event.guild_id) {
                    Some(guild) => match guild
                        .members
                        .write()
                        .expect("Guild inner member cache got poisoned!")
                        .remove(&event.user.id)
                    {
                        Some(member) => {
                            let count = member.user(self).mutual_servers.fetch_sub(1, Ordering::SeqCst) - 1;

                            debug!("{} is now in {} mutual servers", member.user_id, count);
                            if count == 0 {
                                debug!("purging {} from the user cache", member.user_id);
                                self.users
                                    .write()
                                    .expect("Global users cache got poisoned!")
                                    .remove(&member.user_id);
                                self.stats.user_counts.unique.dec();
                            }
                            self.stats.user_counts.total.dec();
                        }
                        None => {
                            if guild.complete.load(Ordering::SeqCst) {
                                gearbot_warn!("Received a member remove event for a member that is not in that guild");
                            } else {
                                info!(
                                    "{} left {} before we got their member chunk",
                                    event.user.id, event.guild_id
                                );
                            }
                        }
                    },
                    None => gearbot_warn!(
                        "Received a member remove for guild {} but no such guild exists in cache",
                        event.guild_id
                    ),
                }
            }

            Event::RoleCreate(event) => match self.get_guild(&event.guild_id) {
                Some(guild) => {
                    guild
                        .roles
                        .write()
                        .expect("Guild inner roles cache got poisoned!")
                        .insert(event.role.id, Arc::new(CachedRole::from_role(&event.role)));
                    self.stats.role_count.inc();
                }
                None => gearbot_warn!(
                    "Received a role create event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            Event::RoleUpdate(event) => match self.get_guild(&event.guild_id) {
                Some(guild) => {
                    guild
                        .roles
                        .write()
                        .expect("Guild inner role cache got poisoned!")
                        .insert(event.role.id, Arc::new(CachedRole::from_role(&event.role)));
                }
                None => gearbot_warn!(
                    "Received a role update event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            Event::RoleDelete(event) => match self.get_guild(&event.guild_id) {
                Some(guild) => {
                    guild
                        .roles
                        .write()
                        .expect("Guild inner roles cache got poisoned!")
                        .remove(&event.role_id);
                    self.stats.role_count.dec();
                }
                None => gearbot_warn!(
                    "Received a role delete event for guild {} but no such guild exists in cache",
                    event.guild_id
                ),
            },

            _ => {}
        };
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
        {
            let mut channels = self
                .guild_channels
                .write()
                .expect("Global guild channels cache got poisoned!");
            let guild_channels = guild.channels.read().expect("Guild inner channels cache got poisoned!");
            for channel in guild_channels.values() {
                channels.remove(&channel.get_id());
            }
            self.stats.channel_count.sub(guild_channels.len() as i64);
        }

        {
            let mut users = self.users.write().expect("Global user cache got poisoned!");
            let members = guild.members.read().expect("Guild inner members cache got poisoned!");
            for member in members.values() {
                match users.get(&member.user_id) {
                    Some(user) => {
                        let count = user.mutual_servers.fetch_sub(1, Ordering::SeqCst) - 1;
                        if count == 0 {
                            users.remove(&member.user_id);
                            self.stats.user_counts.unique.dec();
                        }
                    }
                    None => gearbot_warn!("{} vanished from the user cache!", member.user_id),
                }
            }
            self.stats.user_counts.total.sub(members.len() as i64);
        }

        {
            let mut emoji_cache = self.emoji.write().expect("Global emoji cache got poisoned!");
            for emoji in &guild.emoji {
                emoji_cache.remove(&emoji.id);
            }
        }
        self.stats.emoji_count.sub(guild.emoji.len() as i64);
        self.stats
            .role_count
            .sub(guild.roles.read().expect("Guild inner roles cache got poisoned!").len() as i64);

        self.guilds
            .write()
            .expect("Global guild cache got poisoned!")
            .remove(&guild.id);

        if !guild.complete.load(Ordering::SeqCst) {
            self.stats.guild_counts.partial.dec();
        } else {
            self.stats.guild_counts.loaded.dec();
        }
    }

    pub fn insert_private_channel(&self, private_channel: &PrivateChannel) -> Arc<CachedChannel> {
        let channel = CachedChannel::from_private(private_channel, self);
        let arced = Arc::new(channel);
        if let CachedChannel::DM { receiver, .. } = arced.as_ref() {
            self.dm_channels_by_user
                .write()
                .expect("Global DM channels cache got poisoned!")
                .insert(receiver.id, arced.clone());
        }

        self.private_channels
            .write()
            .expect("Global private channels cache got poisoned!")
            .insert(arced.get_id(), arced.clone());
        arced
    }

    pub fn get_or_insert_user(&self, user: &User) -> Arc<CachedUser> {
        match self.get_user(user.id) {
            Some(user) => user,
            None => {
                let arc = Arc::new(CachedUser::from_user(user));
                self.users
                    .write()
                    .expect("Global users cache got poisoned!")
                    .insert(arc.id, arc.clone());
                self.stats.user_counts.unique.inc();
                arc
            }
        }
    }

    pub fn get_guild(&self, guild_id: &GuildId) -> Option<Arc<CachedGuild>> {
        match self
            .guilds
            .read()
            .expect("Global guild cache got poisoned!")
            .get(guild_id)
        {
            Some(guild) => Some(guild.clone()),
            None => None,
        }
    }

    pub fn get_channel(&self, channel_id: ChannelId) -> Option<Arc<CachedChannel>> {
        match self
            .guild_channels
            .read()
            .expect("Global guild channels cache got poisoned!")
            .get(&channel_id)
        {
            Some(channel) => Some(channel.clone()),
            None => match self
                .private_channels
                .read()
                .expect("Global private channels cache got poisoned!")
                .get(&channel_id)
            {
                Some(channel) => Some(channel.clone()),
                None => None,
            },
        }
    }

    pub fn get_dm_channel_for(&self, user_id: UserId) -> Option<Arc<CachedChannel>> {
        match self
            .dm_channels_by_user
            .read()
            .expect("Global DM channels cache got poisoned!")
            .get(&user_id)
        {
            Some(channel) => Some(channel.clone()),
            None => None,
        }
    }

    pub fn get_user(&self, user_id: UserId) -> Option<Arc<CachedUser>> {
        match self
            .users
            .read()
            .expect("Global users cache got poisoned!")
            .get(&user_id)
        {
            Some(guard) => Some(guard.clone()),
            None => None,
        }
    }

    pub fn get_member(&self, guild_id: &GuildId, user_id: &UserId) -> Option<Arc<CachedMember>> {
        match self
            .guilds
            .read()
            .expect("Global guilds cache got poisoned!")
            .get(guild_id)
        {
            Some(guild) => match guild
                .members
                .read()
                .expect("Guild inner members cache got poisoned!")
                .get(&user_id)
            {
                Some(member) => Some(member.clone()),
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
                self.users
                    .write()
                    .expect("Global users cache got poisoned!")
                    .insert(user.id, user);
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
        self.guild_channels
            .write()
            .expect("Global guild channels cache got poisoned!")
            .clear();
        //we do not want to drag along DM channels, we get guild creates for them when they send a message anyways
        self.private_channels
            .write()
            .expect("Global private channels cache got poisoned!")
            .clear();

        //let's go to hyperspeed
        let mut tasks = vec![];
        let mut user_tasks = vec![];

        //but not yet, collect their work first before they start sabotaging each other again >.>
        let mut work_orders: Vec<Vec<GuildId>> = vec![];

        let mut count = 0;
        let mut list = vec![];

        for guild in self.guilds.read().expect("Global guild cache got poisoned!").values() {
            count += guild
                .members
                .read()
                .expect("Guild inner members cache got poisoned!")
                .len()
                + guild
                    .channels
                    .read()
                    .expect("Guild inner channels cache got poisoned!")
                    .len()
                + guild.emoji.len()
                + guild.roles.read().expect("Guild inner roles cache got poisoned!").len();
            list.push(guild.id);
            if count > 100000 {
                work_orders.push(list);
                list = vec![];
                count = 0;
            }
        }
        if !list.is_empty() {
            work_orders.push(list)
        }
        debug!("Freezing {:?} guilds", self.stats.guild_counts.loaded.get());

        for (i, order) in work_orders.into_iter().enumerate() {
            tasks.push(self._prepare_cold_resume_guild(redis_pool, order, i));
        }
        let guild_chunks = tasks.len();

        future::join_all(tasks).await;

        count = 0;
        let user_chunks = {
            let users = self.users.write().expect("Global users cache got poisoned!");
            let chunks = (users.len() / 100000 + 1) as usize;
            let mut user_work_orders: Vec<Vec<UserId>> = vec![vec![]; chunks];
            for user in users.values() {
                user_work_orders[count % chunks].push(user.id);
                count += 1;
            }

            debug!("Freezing {:?} users", users.len());

            for (i, order) in user_work_orders.into_iter().enumerate().take(chunks) {
                user_tasks.push(self._prepare_cold_resume_user(redis_pool, order, i));
            }
            chunks
        };

        future::join_all(user_tasks).await;
        self.users.write().expect("Global users cache got poisoned!").clear();
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
        {
            let mut guilds = self.guilds.write().expect("Global guilds cache got poisoned!");
            for key in todo {
                let g = guilds.remove(&key).unwrap();
                to_dump.push(ColdStorageGuild::from(g));
            }
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
            let user = self
                .users
                .write()
                .expect("Global user cache got poisoned!")
                .remove(&key)
                .unwrap();

            chunk.push(CachedUser {
                id: user.id,
                username: user.username.clone(),
                discriminator: user.discriminator.clone(),
                avatar: user.avatar.clone(),
                bot_user: user.bot_user,
                system_user: user.system_user,
                public_flags: user.public_flags,
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
            if let Err(e) = result {
                return Err(Error::CacheDefrostError(format!("Failed to defrost users: {}", e)));
            }
        }
        self.stats
            .user_counts
            .unique
            .set(self.users.read().expect("User cache got poisoned!").len() as i64);

        let mut guild_defrosters = Vec::with_capacity(guild_chunks);

        for i in 0..guild_chunks {
            guild_defrosters.push(self.defrost_guilds(redis_pool, i));
        }

        for result in future::join_all(guild_defrosters).await {
            if let Err(e) = result {
                return Err(Error::CacheDefrostError(format!("Failed to defrost guilds: {}", e)));
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
        let mut cached_users = self.users.write().expect("User cache got poisoned!");
        for user in users.drain(..) {
            cached_users.insert(user.id, Arc::new(user));
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

            self.stats
                .role_count
                .add(guild.roles.read().expect("Guild role cache got poisoned!").len() as i64);
            {
                let mut guild_channels = self.guild_channels.write().expect("Guild channels cache got poisoned!");
                for channel in guild
                    .channels
                    .read()
                    .expect("A guilds inner channel cache got poisoned!")
                    .values()
                {
                    guild_channels.insert(channel.get_id(), channel.clone());
                }
                self.stats.channel_count.add(guild_channels.len() as i64);
            }

            {
                let mut emoji = self.emoji.write().expect("Global emoji cache got poisoned!");
                for e in &guild.emoji {
                    emoji.insert(e.id, e.clone());
                }
            }
            self.stats.emoji_count.add(guild.emoji.len() as i64);

            self.stats.user_counts.total.add(
                guild
                    .members
                    .read()
                    .expect("Guild inner members cache got poisoned!")
                    .len() as i64,
            );

            self.guilds
                .write()
                .expect("Global guilds cache got poisoned!")
                .insert(guild.id, Arc::new(guild));
            self.stats.guild_counts.loaded.inc();
        }

        Ok(())
    }

    pub fn shard_cached(&self, shard_id: u64) -> bool {
        match self
            .missing_per_shard
            .read()
            .expect("Global shard state tracking cache got poisoned!")
            .get(&shard_id)
        {
            Some(atomic) => atomic.load(Ordering::Relaxed) == 0,
            None => true, //we cold resumed so have everything
        }
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
