// TODO: Remove this when the bot is a bit more functional
#![allow(dead_code)]

use std::convert::{Infallible, TryFrom};
use std::env;
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use git_version::git_version;
use log::{debug, info};
use tokio::{self, runtime::Runtime, stream::StreamExt, sync::mpsc};
use twilight_gateway::{cluster::ShardScheme, shard::ResumeSession, Cluster, Event};
use twilight_http::{
    client::Proxy, request::channel::message::allowed_mentions::AllowedMentionsBuilder, Client as HttpClient,
};
use twilight_model::{
    gateway::{
        payload::update_status::UpdateStatusInfo,
        presence::{ActivityType, Status},
        Intents,
    },
    user::CurrentUser,
};

use prometheus::{Encoder, TextEncoder};

use crate::core::{logging, logpump, status as bot_status, BotConfig, BotContext, BotStats, ColdRebootData};
use crate::error::{EventHandlerError, StartupError};
use commands::ROOT_NODE;
use translation::Translations;

mod commands;
mod core;

pub mod cache;
use cache::Cache;

mod parser;
pub use parser::Parser;

mod database;
use database::DataStorage;

mod error;
mod handlers;

mod translation;
mod utils;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT_VERSION: &str = git_version!();

#[derive(Debug, Copy, Clone)]
pub struct SchemeInfo {
    pub cluster_id: u64,
    pub shards_per_cluster: u64,
    pub total_shards: u64,
}

fn main() -> Result<(), StartupError> {
    let mut runtime = Runtime::new()?;

    runtime.block_on(async move { real_main().await })?;

    runtime.shutdown_timeout(Duration::from_secs(90));
    Ok(())
}

async fn real_main() -> Result<(), StartupError> {
    println!("Gearbot v{} starting!", VERSION);
    // Read config file
    let config = BotConfig::new(&env::var("CONFIG_FILE").unwrap_or_else(|_| String::from("config.toml")))?;
    println!("Loaded config file");

    let mut builder = HttpClient::builder()
        .token(&config.tokens.discord)
        .default_allowed_mentions(AllowedMentionsBuilder::new().build_solo());
    if let Some(proxy_url) = &config.proxy_url {
        builder = builder
            .proxy(Proxy::all(proxy_url).unwrap())
            .proxy_http(true)
            .ratelimiter(None);
    }

    let http = builder.build()?;
    // Validate token and figure out who we are
    let bot_user = http.current_user().await?;
    info!(
        "Token validated, connecting to discord as {}#{}",
        bot_user.name, bot_user.discriminator
    );

    if let Err(e) = logging::initialize(http.clone(), &config, bot_user.clone()) {
        gearbot_error!("{}", e);
        return Err(e);
    }

    gearbot_important!("Starting Gearbot v{}. Hello there, Ferris!", VERSION);

    let translations = translation::load_translations();
    gearbot_info!("Loaded translations!");

    let datastore = DataStorage::initalize(&config).await?;

    {
        info!("Populating command list");
        ROOT_NODE.all_commands.get("something");
        info!("Command list populated")
    }

    // end of the critical failure zone, everything from here on out should be properly wrapped
    // and handled

    // Parse CLI arguments for sharding and cluster info
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let cluster_id = args
        .get(0)
        .map(|cs| cs.parse::<u64>().unwrap_or_default())
        .unwrap_or_default();
    let shards_per_cluster = args.get(1).map(|spc| spc.parse::<u64>().unwrap_or(1)).unwrap_or(1);
    let total_shards = args.get(2).map(|ts| ts.parse::<u64>().unwrap_or(1)).unwrap_or(1);

    let scheme_info = SchemeInfo {
        cluster_id,
        shards_per_cluster,
        total_shards,
    };

    if let Err(e) = run(scheme_info, config, http, bot_user, datastore, translations).await {
        gearbot_error!("Failed to start the bot: {}", e)
    }

    Ok(())
}

async fn run(
    scheme_info: SchemeInfo,
    config: BotConfig,
    http: HttpClient,
    bot_user: CurrentUser,
    datastore: DataStorage,
    translations: Translations,
) -> Result<(), StartupError> {
    let sharding_scheme = ShardScheme::try_from((
        scheme_info.cluster_id * scheme_info.shards_per_cluster
            ..scheme_info.cluster_id * scheme_info.shards_per_cluster + scheme_info.shards_per_cluster,
        scheme_info.total_shards,
    ))
    .unwrap();

    let intents = Intents::GUILDS
        | Intents::GUILD_MEMBERS
        | Intents::GUILD_BANS
        | Intents::GUILD_EMOJIS
        | Intents::GUILD_INVITES
        | Intents::GUILD_VOICE_STATES
        | Intents::GUILD_MESSAGES
        | Intents::GUILD_MESSAGE_REACTIONS
        | Intents::DIRECT_MESSAGES
        | Intents::DIRECT_MESSAGE_REACTIONS;

    let stats = Arc::new(BotStats::new(scheme_info.cluster_id));
    tokio::spawn(run_metrics_server(Arc::clone(&stats)));

    let cache = Cache::new(scheme_info.cluster_id, Arc::clone(&stats));

    let mut cb = Cluster::builder(&config.tokens.discord, intents)
        .shard_scheme(sharding_scheme)
        .presence(UpdateStatusInfo::new(
            vec![bot_status::generate_activity(
                ActivityType::Listening,
                String::from("to the modem screeching as I connect to the gateway"),
            )],
            true,
            None,
            Status::Idle,
        ));

    // Check for resume data, pass to builder if present
    let key = format!("cb_cluster_data_{}", scheme_info.cluster_id);
    let cache_pool = &datastore.cache_pool;
    match cache_pool.get::<ColdRebootData>(&key).await {
        Ok(result) => {
            if let Some(cold_cache) = result {
                debug!("ColdRebootData: {:?}", cold_cache);

                cache_pool.delete(&key).await?;

                if (cold_cache.total_shards == scheme_info.total_shards)
                    && (cold_cache.shard_count == scheme_info.shards_per_cluster)
                {
                    let map = cold_cache
                        .resume_data
                        .into_iter()
                        .map(|(id, data)| {
                            (
                                id,
                                ResumeSession {
                                    session_id: data.0,
                                    sequence: data.1,
                                },
                            )
                        })
                        .collect();

                    let start = Instant::now();
                    let result = cache
                        .restore_cold_resume(cache_pool, cold_cache.guild_chunks, cold_cache.user_chunks)
                        .await;

                    if let Err(e) = result {
                        gearbot_error!("Cold resume defrosting failed: {}", e);
                        cache.reset();
                    } else {
                        gearbot_important!("Cold resume defrosting completed in {}ms!", start.elapsed().as_millis());
                        cb = cb.resume_sessions(map);
                    }
                }
            }
        }
        Err(e) => {
            gearbot_error!("Failed to get cold resume data: {}", e);
        }
    }

    let cluster = cb.build().await?;

    let (sender, receiver) = mpsc::unbounded_channel();

    let context = Arc::new(BotContext::new(
        (cache, cluster, scheme_info),
        (http, bot_user),
        datastore,
        translations,
        config.global_admins,
        stats,
        sender,
    ));
    let ctx = context.clone();
    let mut _logpump_task = tokio::spawn(logpump::run(ctx, receiver));

    //establish api connection
    let c = context.clone();
    log::debug!("spawning api link");
    tokio::spawn(async move {
        c.datastore.cache_pool.establish_api_link(c.clone()).await;
    });

    let shutdown_ctx = context.clone();
    ctrlc::set_handler(move || {
        // We need a seperate runtime, because at this point in the program,
        // the tokio::main instance isn't running anymore.
        let mut rt = tokio::runtime::Runtime::new().unwrap();
        let _ = rt.block_on(shutdown_ctx.initiate_cold_resume());
        process::exit(0);
    })
    .expect("Failed to register shutdown handler!");

    gearbot_info!("The cluster is going online!");
    let up_cluster = context.cluster.clone();
    tokio::spawn(async move {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        up_cluster.up().await;
    });

    let mut bot_events = context.cluster.events();
    while let Some(event) = bot_events.next().await {
        let c = context.clone();
        context.update_stats(event.0, &event.1).await; //this is fine to await, only async for updating shard states, gona be extremely rare something else also has a lock on that
        context.cache.update(event.0, &event.1, context.clone()).await; //we are awaiting this because cache needs ot be updated before it's safe to spawn off the handling, to avoid working with stale data
        tokio::spawn(async {
            if let Err(e) = handle_event(event, c).await {
                gearbot_error!("{}", e);
            }
        });
    }
    context.cluster.down();

    //TODO: enable when we move to tokio 0.3
    // logpump_task.abort();

    Ok(())
}

async fn handle_event(event: (u64, Event), ctx: Arc<BotContext>) -> Result<(), EventHandlerError> {
    handlers::modlog::handle_event(event.0, &event.1, ctx.clone()).await?;
    handlers::general::handle_event(event.0, &event.1, ctx.clone()).await?;

    // Bot stat handling "hooks". This can be converted into a match if we have more stats to register here.
    if let Event::MessageCreate(msg) = &event.1 {
        ctx.stats.new_message(&ctx, msg).await;
    }

    handlers::commands::handle_event(event.0, event.1, ctx.clone()).await?;

    Ok(())
}

async fn run_metrics_server(stats: Arc<BotStats>) {
    use hyper::service::{make_service_fn, service_fn};
    use hyper::{Body, Response};

    let metric_service = make_service_fn(move |_| {
        let stats = stats.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |_req| {
                let mut buffer = vec![];
                let encoder = TextEncoder::new();
                let metric_families = stats.registry.gather();
                encoder.encode(&metric_families, &mut buffer).unwrap();

                async move { Ok::<_, Infallible>(Response::new(Body::from(buffer))) }
            }))
        }
    });

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 9091));
    let server = hyper::Server::bind(&addr).serve(metric_service);
    if let Err(e) = server.await {
        gearbot_error!("The metrics server failed: {}", e)
    }
}
