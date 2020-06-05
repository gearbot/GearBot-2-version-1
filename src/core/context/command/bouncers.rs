use crate::core::CommandContext;
use crate::utils::Error;
use twilight::model::gateway::presence::{ActivityType, Status};

impl CommandContext {
    pub async fn set_cluster_activity(
        &self,
        status: Status,
        activity_type: ActivityType,
        message: String,
    ) -> Result<(), Error> {
        self.bot_context
            .set_cluster_activity(status, activity_type, message)
            .await
    }

    pub async fn set_shard_activity(
        &self,
        shard_id: u64,
        status: Status,
        activity_type: ActivityType,
        message: String,
    ) -> Result<(), Error> {
        self.bot_context
            .set_shard_activity(shard_id, status, activity_type, message)
            .await
    }

    pub async fn initiate_cold_resume(&self) -> Result<(), Error> {
        self.bot_context.initiate_cold_resume().await
    }
}
