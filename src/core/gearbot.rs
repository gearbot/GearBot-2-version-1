use std::collections::HashMap;
use std::convert::TryFrom;
use std::error;
use std::process;
use std::sync::Arc;
use std::time::Instant;

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
use crate::core::handlers::{commands, general, modlog};
use crate::core::{BotConfig, BotContext, ColdRebootData};
use crate::translation::Translations;
use crate::utils::Error;
use crate::{gearbot_error, gearbot_important, gearbot_info};

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

        let cache = Cache::new(cluster_id);

        let mut cb = ClusterConfig::builder(&config.tokens.discord)
            .shard_scheme(sharding_scheme)
            .intents(intents);

        //check for resume data, pass to builder if present
        let mut connection = redis_pool.get().await;

        let key = format!("cb_cluster_data_{}", cluster_id);
        let data = connection.get(&key).await.unwrap();
        match data {
            Some(d) => {
                let cold_cache: ColdRebootData =
                    serde_json::from_str(&*String::from_utf8(d).unwrap())?;
                debug!("ColdRebootData: {:?}", cold_cache);
                connection
                    .del(format!("cb_cluster_data_{}", cluster_id))
                    .await?;
                if cold_cache.total_shards == total_shards
                    && cold_cache.shard_count == shards_per_cluster
                {
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
                        .restore_cold_resume(
                            &redis_pool,
                            cold_cache.guild_chunks,
                            cold_cache.user_chunks,
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            let end = std::time::Instant::now();
                            gearbot_important!(
                                "Cold resume defrosting completed in {}ms!",
                                (end - start).as_millis()
                            );
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

        let cluster = Cluster::new(cluster_config);
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
        context.cluster.up().await?;
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
        Event::GuildDelete(_) => ctx.stats.left_guild().await,
        _ => {}
    }

    commands::handle_event(event.0, event.1, ctx.clone()).await?;

    Ok(())
}
