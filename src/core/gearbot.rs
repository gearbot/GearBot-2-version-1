use std::collections::HashMap;
use std::convert::TryFrom;
use std::error;
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ctrlc;
use darkredis::ConnectionPool;
use deadpool_postgres::Pool;
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
use crate::{gearbot_error, gearbot_important, gearbot_info};
use prometheus::{Encoder, TextEncoder};
use twilight::model::gateway::payload::update_status::UpdateStatusInfo;
use twilight::model::gateway::payload::UpdateStatus;
use twilight::model::gateway::presence::{ActivityType, Status};
use warp::Filter;

pub struct GearBot;

impl GearBot {
    pub async fn run(
        cluster_id: u64,
        shards_per_cluster: u64,
        total_shards: u64,
        config: BotConfig,
        http: HttpClient,
        user: CurrentUser,
        postgres_pool: Pool,
        redis_pool: ConnectionPool,
        translations: Translations,
    ) -> Result<(), Box<dyn error::Error + Send + Sync>> {
        let sharding_scheme = ShardScheme::try_from((
            cluster_id * shards_per_cluster..cluster_id * shards_per_cluster + shards_per_cluster,
            total_shards,
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

        let stats = Arc::new(BotStats::new(cluster_id));
        let s = stats.clone();
        tokio::spawn(async move {
            let hello = warp::path!("stats").map(move || {
                let mut buffer = vec![];
                let encoder = TextEncoder::new();
                let metric_families = s.registry.gather();
                encoder.encode(&metric_families, &mut buffer).unwrap();
                String::from_utf8(buffer).unwrap()
            });
            let port = 9091 + cluster_id as u16;
            warp::serve(hello).run(([127, 0, 0, 1], port)).await;
        });

        let cache = Cache::new(cluster_id, stats.clone());

        let mut cb = ClusterConfig::builder(&config.tokens.discord)
            .shard_scheme(sharding_scheme)
            .intents(intents)
            .presence(UpdateStatusInfo::new(
                true,
                generate_activity(
                    ActivityType::Listening,
                    String::from("to the modem screeching as i connect to the gateway"),
                ),
                None,
                Status::Idle,
            ));

        //check for resume data, pass to builder if present
        let mut connection = redis_pool.get().await;

        let key = format!("cb_cluster_data_{}", cluster_id);
        let data = connection.get(&key).await.unwrap();
        match data {
            Some(d) => {
                let cold_cache: ColdRebootData = serde_json::from_str(&*String::from_utf8(d).unwrap())?;
                debug!("ColdRebootData: {:?}", cold_cache);
                connection.del(format!("cb_cluster_data_{}", cluster_id)).await?;
                if cold_cache.total_shards == total_shards && cold_cache.shard_count == shards_per_cluster {
                    let mut map = HashMap::new();
                    for (id, data) in cold_cache.resume_data {
                        map.insert(
                            id,
                            ResumeSession {
                                session_id: data.0,
                                sequence: data.1,
                            },
                        );
                    }
                    let start = Instant::now();
                    let result = cache
                        .restore_cold_resume(&redis_pool, cold_cache.guild_chunks, cold_cache.user_chunks)
                        .await;
                    match result {
                        Ok(_) => {
                            let end = std::time::Instant::now();
                            gearbot_important!("Cold resume defrosting completed in {}ms!", (end - start).as_millis());
                            cb = cb.resume_sessions(map);
                        }

                        Err(e) => {
                            gearbot_error!("Cold resume defrosting failed! {}", e);
                            cache.reset();
                        }
                    }
                }
            }

            None => {}
        };
        let cluster_config = cb.build();

        let cluster = Cluster::new(cluster_config).await?;
        let context = Arc::new(BotContext::new(
            cache,
            cluster,
            http,
            user,
            postgres_pool,
            translations,
            config.__master_key,
            redis_pool.clone(),
            cluster_id,
            shards_per_cluster,
            total_shards,
            stats.clone(),
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
            tokio::time::delay_for(Duration::new(1, 0)).await;
            c.up().await;
        });
        let mut bot_events = context.cluster.events().await;
        while let Some(event) = bot_events.next().await {
            let c = context.clone();
            tokio::spawn(async {
                let result = handle_event(event, c).await;
                if result.is_err() {
                    gearbot_error!("{}", result.err().unwrap());
                    // c.stats.had_error().await
                }
            });
        }
        context.cluster.down().await;

        Ok(())
    }
}

async fn handle_event(event: (u64, Event), ctx: Arc<BotContext>) -> Result<(), Error> {
    // Process anything that uses the event ID that we care about, aka shard events
    // debug!("Got a {:?} event on shard {}", event.1.kind(), event.0);
    modlog::handle_event(event.0, &event.1, ctx.clone()).await?;
    general::handle_event(event.0, &event.1, ctx.clone()).await?;

    // Bot stat handling "hooks"
    match &event.1 {
        Event::MessageCreate(msg) => ctx.stats.new_message(&ctx, msg).await,
        _ => {}
    }
    ctx.update_stats(event.0, &event.1);

    commands::handle_event(event.0, event.1, ctx.clone()).await?;

    Ok(())
}
