use std::convert::TryFrom;
use std::error;
use std::sync::Arc;

use log::debug;
use tokio::stream::StreamExt;
use twilight::cache::twilight_cache_inmemory::config::{
    EventType as CacheEventType, InMemoryConfigBuilder,
};
use twilight::cache::InMemoryCache;
use twilight::gateway::cluster::config::ShardScheme;
use twilight::gateway::cluster::Event;
use twilight::gateway::{Cluster, ClusterConfig};
use twilight::http::Client as HttpClient;
use twilight::model::gateway::GatewayIntents;

use crate::core::handlers::{commands, general, modlog};
use crate::core::{BotConfig, Context};
use crate::utils::Error;
use crate::{gearbot_error, gearbot_info};
use deadpool_postgres::Pool;
use twilight::model::user::CurrentUser;

pub struct GearBot;

impl GearBot {
    pub async fn run(
        config: BotConfig,
        http: HttpClient,
        user: CurrentUser,
        pool: Pool,
    ) -> Result<(), Box<dyn error::Error + Send + Sync>> {
        // gearbot_info!("GearBot startup initiated!");
        let sharding_scheme = ShardScheme::try_from((0..2, 2)).unwrap();

        let intents = Some(
            GatewayIntents::GUILDS
                | GatewayIntents::GUILD_MEMBERS
                | GatewayIntents::GUILD_BANS
                | GatewayIntents::GUILD_EMOJIS
                | GatewayIntents::GUILD_INVITES
                | GatewayIntents::GUILD_VOICE_STATES
                | GatewayIntents::GUILD_MESSAGES
                | GatewayIntents::GUILD_MESSAGE_REACTIONS
                | GatewayIntents::DIRECT_MESSAGES
                | GatewayIntents::DIRECT_MESSAGE_REACTIONS,
        );

        let cluster_config = ClusterConfig::builder(&config.tokens.discord)
            .shard_scheme(sharding_scheme)
            .intents(intents)
            .build();

        let cache_config = InMemoryConfigBuilder::new()
            .event_types(
                CacheEventType::MESSAGE_CREATE
                    | CacheEventType::MESSAGE_DELETE
                    | CacheEventType::MESSAGE_DELETE_BULK
                    | CacheEventType::MESSAGE_UPDATE
                    | CacheEventType::CHANNEL_CREATE
                    | CacheEventType::CHANNEL_DELETE
                    | CacheEventType::CHANNEL_UPDATE
                    | CacheEventType::GUILD_CREATE
                    | CacheEventType::GUILD_DELETE
                    | CacheEventType::GUILD_EMOJIS_UPDATE
                    | CacheEventType::GUILD_UPDATE
                    | CacheEventType::MEMBER_ADD
                    | CacheEventType::MEMBER_CHUNK
                    | CacheEventType::MEMBER_REMOVE
                    | CacheEventType::MEMBER_UPDATE
                    | CacheEventType::MESSAGE_CREATE
                    | CacheEventType::MESSAGE_DELETE
                    | CacheEventType::MESSAGE_DELETE_BULK
                    | CacheEventType::MESSAGE_UPDATE
                    | CacheEventType::REACTION_ADD
                    | CacheEventType::REACTION_REMOVE
                    | CacheEventType::REACTION_REMOVE_ALL
                    | CacheEventType::ROLE_CREATE
                    | CacheEventType::ROLE_DELETE
                    | CacheEventType::ROLE_UPDATE
                    | CacheEventType::UNAVAILABLE_GUILD
                    | CacheEventType::UPDATE_VOICE_STATE
                    | CacheEventType::VOICE_SERVER_UPDATE
                    | CacheEventType::VOICE_STATE_UPDATE
                    | CacheEventType::WEBHOOKS_UPDATE,
            )
            .build();

        let cache = InMemoryCache::from(cache_config);
        let cluster = Cluster::new(cluster_config);
        let context = Arc::new(Context::new(
            cache,
            cluster,
            http,
            user,
            pool,
            config.__master_key,
        ));

        gearbot_info!("The cluster is going online!");
        let mut bot_events = context.cluster.events().await?;
        while let Some(event) = bot_events.next().await {
            let c = context.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_event(event, c.clone()).await {
                    gearbot_error!("{}", e);
                    c.stats.had_error().await;
                }
            });
        }

        Ok(())
    }
}

async fn handle_event(event: (u64, Event), ctx: Arc<Context>) -> Result<(), Error> {
    // Process anything that uses the event ID that we care about, aka shard events
    debug!(
        "Got a {:?} event on shard {}",
        event.1.event_type(),
        event.0
    );
    modlog::handle_event(event.0, &event.1, ctx.clone()).await?;
    general::handle_event(event.0, &event.1, ctx.clone()).await?;

    // Bot stat handling "hooks"
    match &event.1 {
        Event::MessageCreate(msg) => ctx.stats.new_message(&ctx, msg).await,
        Event::GuildDelete(_) => ctx.stats.left_guild().await,
        _ => {}
    }

    commands::handle_event(event.0, event.1, ctx.clone()).await?;

    Ok(())
}
