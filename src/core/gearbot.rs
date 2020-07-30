use std::convert::{Infallible, TryFrom};
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::debug;
use tokio::{self, stream::StreamExt};
use twilight::gateway::cluster::config::ShardScheme;
use twilight::gateway::shard::ResumeSession;
use twilight::gateway::{Cluster, ClusterConfig, Event};
use twilight::http::Client as HttpClient;
use twilight::model::gateway::GatewayIntents;
use twilight::model::user::CurrentUser;

use crate::core::cache::Cache;
use crate::core::context::bot::status::generate_activity;
use crate::core::handlers::{commands, general, modlog};
use crate::core::{BotConfig, BotContext, BotStats, ColdRebootData};
use crate::translation::Translations;
use crate::utils::Error;
use crate::{gearbot_error, gearbot_important, gearbot_info, SchemeInfo};
use prometheus::{Encoder, TextEncoder};
use twilight::model::gateway::payload::update_status::UpdateStatusInfo;
use twilight::model::gateway::presence::{ActivityType, Status};

pub async fn run(
    scheme_info: SchemeInfo,
    config: BotConfig,
    http: HttpClient,
    bot_user: CurrentUser,
    postgres_pool: sqlx::PgPool,
    redis_pool: darkredis::ConnectionPool,
    translations: Translations,
) -> Result<(), Error> {
    let sharding_scheme = ShardScheme::try_from((
        scheme_info.cluster_id * scheme_info.shards_per_cluster
            ..scheme_info.cluster_id * scheme_info.shards_per_cluster + scheme_info.shards_per_cluster,
        scheme_info.total_shards,
    ))
    .unwrap();

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

    let stats = Arc::new(BotStats::new(scheme_info.cluster_id));
    let s = stats.clone();
    tokio::spawn(run_metrics_server(s, scheme_info.cluster_id));

    let cache = Cache::new(scheme_info.cluster_id, stats.clone());

    let mut cb = ClusterConfig::builder(&config.tokens.discord)
        .shard_scheme(sharding_scheme)
        .intents(intents)
        .presence(UpdateStatusInfo::new(
            true,
            generate_activity(
                ActivityType::Listening,
                String::from("to the modem screeching as I connect to the gateway"),
            ),
            None,
            Status::Idle,
        ));

    // Check for resume data, pass to builder if present
    {
        let mut redis_conn = redis_pool.get().await;
        let key = format!("cb_cluster_data_{}", scheme_info.cluster_id);
        if let Some(cache_data) = redis_conn.get(&key).await.unwrap() {
            let cold_cache: ColdRebootData = serde_json::from_str(&String::from_utf8(cache_data).unwrap())?;
            debug!("ColdRebootData: {:?}", cold_cache);

            redis_conn
                .del(format!("cb_cluster_data_{}", scheme_info.cluster_id))
                .await?;
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
                    .restore_cold_resume(&redis_pool, cold_cache.guild_chunks, cold_cache.user_chunks)
                    .await;

                if let Err(e) = result {
                    gearbot_error!("Cold resume defrosting failed: {}", e);
                    cache.reset();
                } else {
                    gearbot_important!("Cold resume defrosting completed in {}ms!", start.elapsed().as_millis());
                    cb = cb.resume_sessions(map);
                }
            }
        };
    }

    let cluster = Cluster::new(cb.build()).await?;
    let context = Arc::new(BotContext::new(
        (cache, cluster, scheme_info),
        (http, bot_user),
        (postgres_pool, redis_pool),
        translations,
        (config.__master_key, config.global_admins),
        stats,
    ));

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
    let c = context.cluster.clone();
    tokio::spawn(async move {
        tokio::time::delay_for(Duration::from_secs(1)).await;
        c.up().await;
    });

    let mut bot_events = context.cluster.events().await;
    while let Some(event) = bot_events.next().await {
        let c = context.clone();
        context.update_stats(event.0, &event.1);
        context.cache.update(event.0, &event.1, context.clone());
        tokio::spawn(async {
            if let Err(e) = handle_event(event, c).await {
                gearbot_error!("{}", e);
            }
        });
    }
    context.cluster.down().await;

    Ok(())
}

async fn handle_event(event: (u64, Event), ctx: Arc<BotContext>) -> Result<(), Error> {
    // Process anything that uses the event ID that we care about, aka shard events
    // debug!("Got a {:?} event on shard {}", event.1.kind(), event.0);
    modlog::handle_event(event.0, &event.1, ctx.clone()).await?;
    general::handle_event(event.0, &event.1, ctx.clone()).await?;

    // Bot stat handling "hooks". This can be converted into a match if we have more stats to register here.
    if let Event::MessageCreate(msg) = &event.1 {
        ctx.stats.new_message(&ctx, msg).await;
    }

    commands::handle_event(event.0, event.1, ctx.clone()).await?;

    Ok(())
}

async fn run_metrics_server(stats: Arc<BotStats>, cluster_id: u64) {
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

    let port = 9091 + cluster_id as u16;
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let server = hyper::Server::bind(&addr).serve(metric_service);
    if let Err(e) = server.await {
        gearbot_error!("The metrics server failed: {}", e)
    }
}
