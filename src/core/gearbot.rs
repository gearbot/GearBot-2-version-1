use std::error;
use std::sync::Arc;

use log::info;
use tokio::stream::StreamExt;
use twilight::cache::InMemoryCache;
use twilight::cache::twilight_cache_inmemory::config::{EventType as CacheEventType, InMemoryConfigBuilder};
use twilight::command_parser::{CommandParserConfig, Parser};
use twilight::gateway::{Cluster, ClusterConfig};
use twilight::gateway::cluster::config::ShardScheme;
use twilight::gateway::cluster::Event;
use twilight::http::{Client as HttpClient, Client};
use twilight::model::gateway::GatewayIntents;
use twilight::model::id::WebhookId;

use crate::{COMMAND_LIST, CommandResult, Error, gearbot_info};
use crate::core::{BotConfig, Context};
use crate::gears::basic;

pub struct GearBot<'a> {
    config: BotConfig,
    context: Arc<Context<'a>>,
}

impl GearBot<'_> {
    pub async fn run(config: BotConfig, http: Client) -> Result<(), Box<dyn error::Error + Send + Sync>> {
        // gearbot_info!("GearBot startup initiated!");
        let sharding_scheme = ShardScheme::Auto;

        let intents = Some(
            GatewayIntents::GUILDS |
                GatewayIntents::GUILD_MEMBERS |
                GatewayIntents::GUILD_BANS |
                GatewayIntents::GUILD_EMOJIS |
                GatewayIntents::GUILD_INVITES |
                GatewayIntents::GUILD_VOICE_STATES |
                GatewayIntents::GUILD_MESSAGES |
                GatewayIntents::GUILD_MESSAGE_REACTIONS |
                GatewayIntents::DIRECT_MESSAGES |
                GatewayIntents::DIRECT_MESSAGE_REACTIONS
        );

        let cluster_config = ClusterConfig::builder(&config.tokens.discord)
            .shard_scheme(sharding_scheme)
            .intents(intents)
            .build();

        let cache_config = InMemoryConfigBuilder::new()
            .event_types(
                CacheEventType::MESSAGE_CREATE |
                    CacheEventType::MESSAGE_DELETE |
                    CacheEventType::MESSAGE_DELETE_BULK |
                    CacheEventType::MESSAGE_UPDATE
            )
            .build();

        let cache = InMemoryCache::from(cache_config);

//TODO: autogen and move to own section
        let cmd_parser = {
            let mut commands_config = CommandParserConfig::new();
            commands_config.add_prefix("?");
            for cmd in &COMMAND_LIST {
                commands_config.command(*cmd).case_insensitive().add()
            }
            Parser::new(commands_config)
        };


        gearbot_info!("Cluster going online!");
        let cluster = Cluster::new(cluster_config);
        cluster.up().await?;

        let context = Arc::new(Context::new(
            cmd_parser,
            cache,
            cluster,
            http,
        ));

        let mut bot_events = context.cluster.events().await;
        while let Some(event) = bot_events.next().await {
            context.cache.update(&event.1).await;


            tokio::spawn( handle_event(event, context.clone()) );
        }


        Ok(())
    }


}

// TODO: Fix the silly default error handling
async fn handle_event(event: (u64, Event), ctx: Arc<Context<'_>>) -> Result<(), Error> {
    // Process anything that uses the event ID that we care about
    match &event {
        (id, Event::ShardConnected(_)) => gearbot_info!("Shard {} has connected", id),
        (id, Event::ShardDisconnected(_)) => gearbot_info!("Shard {} has disconnected", id),
        (id, Event::ShardReconnecting(_)) => gearbot_info!("Shard {} is attempting to reconnect", id),
        (id, Event::ShardResuming(_)) => gearbot_info!("Shard {} is resuming itself", id),
        _ => ()
    }

    // Since we handled anything with a id we care about, we can make the
    // next match simpler.
    let event = event.1;
    match event {
        Event::MessageCreate(msg) => {
            info!("Received a message from {}, saying {}", msg.author.name, msg.content);
            if let Some(command) = ctx.command_parser.parse(&msg.content) {
                let args = command.arguments.as_str();
                match command.name {
                    "ping" => basic::ping(&ctx, &msg).await?,
                    "about" => basic::about(&ctx, &msg).await?,
                    "echo" => basic::echo(&ctx, &msg, args).await?,
                    _ => Ok(())?
                }
            }
            Ok(())?
        }
        _ => Ok(())?
    }

    Ok(())
}