use std::collections::HashMap;

use log::info;
use twilight::model::gateway::presence::{ActivityType, Status};

use crate::core::{BotContext, ColdRebootData};
use crate::gearbot_important;
use crate::utils::Error;

impl BotContext {
    pub async fn initiate_cold_resume(&self) -> Result<(), Error> {
        // preparing for update rollout, set status to atleast give some indication to users
        gearbot_important!("Preparing for cold resume!");
        self.set_cluster_activity(
            Status::Idle,
            ActivityType::Watching,
            String::from("the new update being deployed. Replies might be delayed a bit"),
        )
        .await?;

        let start = std::time::Instant::now();

        let mut connection = self.redis_pool.get().await;

        //kill the shards and get their resume info
        //DANGER: WE WILL NOT BE GETTING EVENTS FROM THIS POINT ONWARDS, REBOOT REQUIRED

        info!("Resume data acquired");

        let resume_data = self.cluster.down_resumable().await;
        let (guild_chunks, user_chunks) = self.cache.prepare_cold_resume(&self.redis_pool).await;

        // prepare resume data
        let mut map = HashMap::with_capacity(resume_data.len());
        for (shard_id, data) in resume_data {
            map.insert(shard_id, (data.session_id, data.sequence));
        }

        let data = ColdRebootData {
            resume_data: map,
            total_shards: self.total_shards,
            guild_chunks,
            shard_count: self.shards_per_cluster,
            user_chunks,
        };

        connection
            .set_and_expire_seconds(
                format!("cb_cluster_data_{}", self.cluster_id),
                &serde_json::to_value(data).unwrap().to_string().into_bytes(),
                180,
            )
            .await
            .unwrap();

        let end = std::time::Instant::now();
        info!("Cold resume preparations completed in {}ms!", (end - start).as_millis());

        Ok(())
    }
}
