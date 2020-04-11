use dotenv;
use log::{error, info};
use pretty_env_logger;
use tokio::stream::StreamExt;
use std::sync::Arc;
use twilight::{
    cache::{
        twilight_cache_inmemory::config::{ConfigBuilder as CacheConfig, EventType as CacheEventType},
        InMemoryCache,
    },
    gateway::{cluster::{config::ShardScheme, Cluster, Config as ClusterConfig}, shard::Event},
    http::Client as HttpClient,
    model::gateway::GatewayIntents,
    command_parser::{Config as ParserConfig, Parser},
};
use std::error;

mod gears;
use gears::basic;

pub enum Error {
    SomethingBadHappened,
    MissingToken,
}

pub type CommandResult = Result<(), Error>;

#[derive(Debug)]
pub struct Context<'a> {
    command_parser: Parser<'a>,
    cache: InMemoryCache,
    cluster: Cluster,
    http: HttpClient,
}

impl<'a> Context<'a> {
    fn new(
        parser: Parser<'a>,
        cache: InMemoryCache,
        cluster: Cluster,
        http: HttpClient
    ) -> Self {
        Context {
            command_parser: parser,
            cache,
            cluster,
            http,
        }
    }
}

const COMMAND_LIST: [&str; 3] = [
    "about",
    "ping",
    "echo",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error + Send + Sync>> {
    pretty_env_logger::init_timed();

    info!("Starting Gearbot. Hello there, Ferris!");

    let token = if let Some(token) = dotenv::var("token").ok() {
        token
    } else {
        error!("The bot token is missing from the enviroment, aborting startup!");
        return Ok(());
    };

    let sharding_scheme = ShardScheme::Auto;

    let intents = Some(
        GatewayIntents::GUILD_MESSAGES |
        GatewayIntents::DIRECT_MESSAGES
    );

    let cluster_config = ClusterConfig::builder(&token)
        .shard_scheme(sharding_scheme)
        .intents(intents)
        .build();

    let cluster = Cluster::new(cluster_config);
    cluster.up().await?;

    let http = HttpClient::new(token);

    let cache_config = CacheConfig::new()
        .event_types(
            CacheEventType::MESSAGE_CREATE |
            CacheEventType::MESSAGE_DELETE |
            CacheEventType::MESSAGE_DELETE_BULK |
            CacheEventType::MESSAGE_UPDATE,
        )
        .build();

    let cache = InMemoryCache::from(cache_config);

    let cmd_parser = {
        let mut commands_config = ParserConfig::new();
        commands_config.add_prefix("?");
        for cmd in &COMMAND_LIST {
            commands_config.command(*cmd).case_insensitive().add()
        }
        Parser::new(commands_config)
    };

    
    // In the future, this will need to be a RwLock when there is a database, etc
    let context = Arc::new(Context::new(
        cmd_parser,
        cache, 
        cluster,    
        http
    ));

    let mut bot_events = context.cluster.events().await;
    while let Some(event) = bot_events.next().await {
        context.cache.update(&event.1).await?;

        tokio::spawn(handle_event(event, context.clone()));
    }

    Ok(())
}

// TODO: Fix the silly default error handling
async fn handle_event(event: (u64, Event), ctx: Arc<Context<'_>>) -> Result<(), Error> {
    // Process anything that uses the event ID that we care about
    match &event {
        (id, Event::ShardConnected(_)) => info!("Shard {} has connected", id),
        (id, Event::ShardDisconnected(_)) => info!("Shard {} has disconnected", id),
        (id, Event::ShardReconnecting(_)) => info!("Shard {} is attempting to reconnect", id),
        (id, Event::ShardResuming(_)) => info!("Shard {} is resuming itself", id),
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
        },
        _ => Ok(())?
    }

    Ok(())
}
