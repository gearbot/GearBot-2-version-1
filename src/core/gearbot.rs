use std::convert::{Infallible, TryFrom};
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::debug;
use prometheus::{Encoder, TextEncoder};
use tokio::{self, stream::StreamExt};
use twilight_gateway::{cluster::ShardScheme, shard::ResumeSession, Cluster, Event};

use twilight_http::Client as HttpClient;
use twilight_model::{
    gateway::{
        payload::update_status::UpdateStatusInfo,
        presence::{ActivityType, Status},
        Intents,
    },
    user::CurrentUser,
};

use crate::core::cache::Cache;
use crate::core::context::bot::status as bot_status;
use crate::core::handlers::{commands, general, modlog};
use crate::core::{logpump, BotConfig, BotContext, BotStats, ColdRebootData};
use crate::database::DataStorage;
use crate::error::{EventHandlerError, StartupError};
use crate::translation::Translations;
use crate::{gearbot_error, gearbot_important, gearbot_info, SchemeInfo};
use tokio::sync::mpsc::unbounded_channel;

pub async fn run(
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

    let cache = Cache::new(scheme_info.cluster_id, stats.clone());

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

    let (sender, receiver) = unbounded_channel();

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
    let mut logpump_task = tokio::spawn(logpump::run(ctx, receiver));

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
    modlog::handle_event(event.0, &event.1, ctx.clone()).await?;
    general::handle_event(event.0, &event.1, ctx.clone()).await?;

    // Bot stat handling "hooks". This can be converted into a match if we have more stats to register here.
    if let Event::MessageCreate(msg) = &event.1 {
        ctx.stats.new_message(&ctx, msg).await;
    }

    commands::handle_event(event.0, event.1, ctx.clone()).await?;

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
