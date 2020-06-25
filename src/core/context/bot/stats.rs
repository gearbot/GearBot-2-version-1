use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::{DateTime, Utc};
use twilight::model::channel::Message;

use git_version::git_version;

use crate::core::BotContext;
use prometheus::{Encoder, IntCounter, IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry, TextEncoder};

use twilight::model::gateway::event::Event;
use warp::Filter;

pub struct EventStats {
    pub ban_add: IntCounter,
    pub ban_remove: IntCounter,
    pub channel_create: IntCounter,
    pub channel_delete: IntCounter,
    pub gateway_reconnect: IntCounter,
    pub channel_pins_update: IntCounter,
    pub guild_create: IntCounter,
    pub guild_delete: IntCounter,
    pub guild_emojis_update: IntCounter,
    pub guild_integrations_update: IntCounter,
    pub guild_update: IntCounter,
    pub invite_create: IntCounter,
    pub invite_delete: IntCounter,
    pub member_add: IntCounter,
    pub member_remove: IntCounter,
    pub member_update: IntCounter,
    pub member_chunk: IntCounter,
    pub message_create: IntCounter,
    pub message_delete: IntCounter,
    pub message_delete_bulk: IntCounter,
    pub message_update: IntCounter,
    pub presence_update: IntCounter,
    pub presences_replace: IntCounter,
    pub reaction_add: IntCounter,
    pub reaction_remove: IntCounter,
    pub reaction_remove_all: IntCounter,
    pub reaction_remove_emoji: IntCounter,
    pub role_create: IntCounter,
    pub role_delete: IntCounter,
    pub role_update: IntCounter,
    pub typing_start: IntCounter,
    pub unavailable_guild: IntCounter,
    pub user_update: IntCounter,
    pub voice_server_update: IntCounter,
    pub voice_state_update: IntCounter,
    pub webhooks_update: IntCounter,
}

pub struct MessageCounters {
    pub user_messages: IntCounter,
    pub other_bot_messages: IntCounter,
    pub own_messages: IntCounter,
    pub webhook_messages: IntCounter,
}

pub struct UserCounters {
    pub unique: IntGauge,
    pub total: IntGauge,
}

pub struct GuildCounters {
    pub partial: IntGauge,
    pub loaded: IntGauge,
    pub outage: IntGauge,
}

pub struct BotStats {
    pub registry: Registry,
    pub start_time: DateTime<Utc>,
    pub version: &'static str,
    pub event_counts: EventStats,
    pub message_counts: MessageCounters,
    pub user_counts: UserCounters,
    pub channel_count: IntGauge,
    pub guild_counts: GuildCounters,
    pub emoji_count: IntGauge,
    pub role_count: IntGauge,
}

impl BotStats {
    pub fn new(cluster_id: u64) -> Self {
        let cid = &*cluster_id.to_string();
        let event_counter = IntCounterVec::new(
            Opts::new("gateway_events", "Events received from the gateway").const_label("cluster", cid),
            &["events"],
        )
        .unwrap();
        let message_counter = IntCounterVec::new(Opts::new("messages", "Recieved messages").const_label("cluster", cid), &["sender_type"]).unwrap();
        let channel_count = IntGauge::with_opts(Opts::new("channels", "Channel count").const_label("cluster", cid)).unwrap();
        let emoji_count = IntGauge::with_opts(Opts::new("emoji", "Emoji count").const_label("cluster", cid)).unwrap();
        let role_count = IntGauge::with_opts(Opts::new("roles", "Role count").const_label("cluster", cid)).unwrap();
        let guild_counter = IntGaugeVec::new(Opts::new("guild_counts", "State of the guilds").const_label("cluster", cid), &["state"]).unwrap();
        let user_counter = IntGaugeVec::new(Opts::new("user_counts", "User counts").const_label("cluster", cid), &["type"]).unwrap();
        let registry = Registry::new();
        registry.register(Box::new(event_counter.clone())).unwrap();
        registry.register(Box::new(message_counter.clone())).unwrap();
        registry.register(Box::new(channel_count.clone())).unwrap();
        registry.register(Box::new(emoji_count.clone())).unwrap();
        registry.register(Box::new(role_count.clone())).unwrap();
        registry.register(Box::new(guild_counter.clone())).unwrap();
        registry.register(Box::new(user_counter.clone())).unwrap();
        BotStats {
            registry,
            start_time: Utc::now(),
            version: git_version!(),
            event_counts: EventStats {
                ban_add: event_counter.get_metric_with_label_values(&["BanAdd"]).unwrap(),
                ban_remove: event_counter.get_metric_with_label_values(&["BanRemove"]).unwrap(),
                channel_create: event_counter.get_metric_with_label_values(&["ChannelCreate"]).unwrap(),
                channel_delete: event_counter.get_metric_with_label_values(&["ChannelDelete"]).unwrap(),
                gateway_reconnect: event_counter.get_metric_with_label_values(&["GatewayReconnect"]).unwrap(),
                channel_pins_update: event_counter.get_metric_with_label_values(&["ChannelPinsUpdate"]).unwrap(),
                guild_create: event_counter.get_metric_with_label_values(&["GuildCreate"]).unwrap(),
                guild_delete: event_counter.get_metric_with_label_values(&["GuildDelete"]).unwrap(),
                guild_emojis_update: event_counter.get_metric_with_label_values(&["GuildEmojisUpdate"]).unwrap(),
                guild_integrations_update: event_counter.get_metric_with_label_values(&["GuildIntegrationsUpdate"]).unwrap(),
                guild_update: event_counter.get_metric_with_label_values(&["GuildUpdate"]).unwrap(),
                invite_create: event_counter.get_metric_with_label_values(&["InviteCreate"]).unwrap(),
                invite_delete: event_counter.get_metric_with_label_values(&["InviteDelete"]).unwrap(),
                member_add: event_counter.get_metric_with_label_values(&["MemberAdd"]).unwrap(),
                member_remove: event_counter.get_metric_with_label_values(&["MemberRemove"]).unwrap(),
                member_update: event_counter.get_metric_with_label_values(&["MemberUpdate"]).unwrap(),
                member_chunk: event_counter.get_metric_with_label_values(&["MemberChunk"]).unwrap(),
                message_create: event_counter.get_metric_with_label_values(&["MessageCreate"]).unwrap(),
                message_delete: event_counter.get_metric_with_label_values(&["MessageDelete"]).unwrap(),
                message_delete_bulk: event_counter.get_metric_with_label_values(&["MessageDeleteBulk"]).unwrap(),
                message_update: event_counter.get_metric_with_label_values(&["MessageUpdate"]).unwrap(),
                presence_update: event_counter.get_metric_with_label_values(&["PresenceUpdate"]).unwrap(),
                presences_replace: event_counter.get_metric_with_label_values(&["PresencesReplace"]).unwrap(),
                reaction_add: event_counter.get_metric_with_label_values(&["ReactionAdd"]).unwrap(),
                reaction_remove: event_counter.get_metric_with_label_values(&["ReactionRemove"]).unwrap(),
                reaction_remove_all: event_counter.get_metric_with_label_values(&["ReactionRemoveAll"]).unwrap(),
                reaction_remove_emoji: event_counter.get_metric_with_label_values(&["ReactionRemoveEmoji"]).unwrap(),
                role_create: event_counter.get_metric_with_label_values(&["RoleCreate"]).unwrap(),
                role_delete: event_counter.get_metric_with_label_values(&["RoleDelete"]).unwrap(),
                role_update: event_counter.get_metric_with_label_values(&["RoleUpdate"]).unwrap(),
                typing_start: event_counter.get_metric_with_label_values(&["TypingStart"]).unwrap(),
                unavailable_guild: event_counter.get_metric_with_label_values(&["UnavailableGuild"]).unwrap(),
                user_update: event_counter.get_metric_with_label_values(&["UserUpdate"]).unwrap(),
                voice_server_update: event_counter.get_metric_with_label_values(&["VoiceServerUpdate"]).unwrap(),
                voice_state_update: event_counter.get_metric_with_label_values(&["VoiceStateUpdate"]).unwrap(),
                webhooks_update: event_counter.get_metric_with_label_values(&["WebhooksUpdate"]).unwrap(),
            },
            message_counts: MessageCounters {
                user_messages: message_counter.get_metric_with_label_values(&["user"]).unwrap(),
                other_bot_messages: message_counter.get_metric_with_label_values(&["bot"]).unwrap(),
                own_messages: message_counter.get_metric_with_label_values(&["own"]).unwrap(),
                webhook_messages: message_counter.get_metric_with_label_values(&["webhook"]).unwrap(),
            },
            user_counts: UserCounters {
                unique: user_counter.get_metric_with_label_values(&["unique"]).unwrap(),
                total: user_counter.get_metric_with_label_values(&["total"]).unwrap(),
            },
            guild_counts: GuildCounters {
                partial: guild_counter.get_metric_with_label_values(&["partial"]).unwrap(),
                loaded: guild_counter.get_metric_with_label_values(&["loaded"]).unwrap(),
                outage: guild_counter.get_metric_with_label_values(&["outage"]).unwrap(),
            },
            channel_count,
            emoji_count,
            role_count,
        }
    }

    pub fn receive_event(&self, event: &Event) {
        match event {
            Event::BanAdd(_) => self.event_counts.ban_add.inc(),
            Event::BanRemove(_) => self.event_counts.ban_remove.inc(),
            Event::ChannelCreate(_) => self.event_counts.channel_create.inc(),
            Event::ChannelDelete(_) => self.event_counts.channel_delete.inc(),
            Event::GatewayReconnect => self.event_counts.gateway_reconnect.inc(),
            Event::ChannelPinsUpdate(_) => self.event_counts.channel_pins_update.inc(),
            Event::GuildCreate(_) => self.event_counts.guild_create.inc(),
            Event::GuildDelete(_) => self.event_counts.guild_delete.inc(),
            Event::GuildEmojisUpdate(_) => self.event_counts.guild_emojis_update.inc(),
            Event::GuildIntegrationsUpdate(_) => self.event_counts.guild_integrations_update.inc(),
            Event::GuildUpdate(_) => self.event_counts.guild_update.inc(),
            Event::InviteCreate(_) => self.event_counts.invite_create.inc(),
            Event::InviteDelete(_) => self.event_counts.invite_delete.inc(),
            Event::MemberAdd(_) => self.event_counts.member_add.inc(),
            Event::MemberRemove(_) => self.event_counts.member_remove.inc(),
            Event::MemberUpdate(_) => self.event_counts.member_update.inc(),
            Event::MemberChunk(_) => self.event_counts.member_chunk.inc(),
            Event::MessageCreate(message) => self.event_counts.message_create.inc(),
            Event::MessageDelete(_) => self.event_counts.message_delete.inc(),
            Event::MessageDeleteBulk(_) => self.event_counts.message_delete_bulk.inc(),
            Event::MessageUpdate(_) => self.event_counts.message_update.inc(),
            Event::PresenceUpdate(_) => self.event_counts.presence_update.inc(),
            Event::PresencesReplace => self.event_counts.presences_replace.inc(),
            Event::ReactionAdd(_) => self.event_counts.reaction_add.inc(),
            Event::ReactionRemove(_) => self.event_counts.reaction_remove.inc(),
            Event::ReactionRemoveAll(_) => self.event_counts.reaction_remove_all.inc(),
            Event::ReactionRemoveEmoji(_) => self.event_counts.reaction_remove_emoji.inc(),
            Event::RoleCreate(_) => self.event_counts.role_create.inc(),
            Event::RoleDelete(_) => self.event_counts.role_delete.inc(),
            Event::RoleUpdate(_) => self.event_counts.role_update.inc(),
            Event::TypingStart(_) => self.event_counts.typing_start.inc(),
            Event::UnavailableGuild(_) => self.event_counts.unavailable_guild.inc(),
            Event::UserUpdate(_) => self.event_counts.user_update.inc(),
            Event::VoiceServerUpdate(_) => self.event_counts.voice_server_update.inc(),
            Event::VoiceStateUpdate(_) => self.event_counts.voice_state_update.inc(),
            Event::WebhooksUpdate(_) => self.event_counts.webhooks_update.inc(),
            _ => {}
        }
    }

    pub async fn new_message(&self, ctx: &BotContext, msg: &Message) {
        if msg.author.bot {
            // This will simply skip incrementing it if we couldn't get
            // a lock on the cache. No harm done.
            if ctx.is_own(msg) {
                self.message_counts.own_messages.inc()
            } else if msg.webhook_id.is_some() {
                self.message_counts.webhook_messages.inc()
            } else {
                self.message_counts.other_bot_messages.inc()
            }
        } else {
            self.message_counts.user_messages.inc()
        }
    }
}

#[derive(Debug)]
pub struct LoadingState {
    to_load: u32,
    loaded: u32,
}
