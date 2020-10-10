use crate::core::BotContext;
use crate::database::redis::api_structs::ReplyData;
use crate::utils::ApiMessageError;
use std::sync::Arc;

pub async fn get_team_info(ctx: Arc<BotContext>) -> Result<ReplyData, ApiMessageError> {
    Ok(ReplyData::TeamInfo(ctx.get_team_info().await))
}
