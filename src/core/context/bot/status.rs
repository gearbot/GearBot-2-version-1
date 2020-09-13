use twilight_model::gateway::payload::UpdateStatus;
use twilight_model::gateway::presence::{Activity, ActivityType, Status};

use crate::core::BotContext;
use crate::utils::Error;

impl BotContext {
    pub async fn set_cluster_activity(
        &self,
        status: Status,
        activity_type: ActivityType,
        message: String,
    ) -> Result<(), Error> {
        for shard_id in self.scheme_info.cluster_id * self.scheme_info.shards_per_cluster
            ..self.scheme_info.cluster_id * self.scheme_info.shards_per_cluster + self.scheme_info.shards_per_cluster
        {
            self.set_shard_activity(shard_id, status, activity_type, message.clone())
                .await?;
        }
        Ok(())
    }

    pub async fn set_shard_activity(
        &self,
        shard_id: u64,
        status: Status,
        activity_type: ActivityType,
        message: String,
    ) -> Result<(), Error> {
        self.cluster
            .command(
                shard_id,
                &UpdateStatus::new(false, generate_activity(activity_type, message), None, status),
            )
            .await?;
        Ok(())
    }
}

pub fn generate_activity(activity_type: ActivityType, message: String) -> Activity {
    Activity {
        assets: None,
        application_id: None,
        created_at: None,
        details: None,
        flags: None,
        id: None,
        instance: None,
        kind: activity_type,
        name: message,
        emoji: None,
        party: None,
        secrets: None,
        state: None,
        timestamps: None,
        url: None,
    }
}
