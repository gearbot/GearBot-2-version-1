use crate::core::BotContext;
use crate::database::redis::api_handlers::api_structs::{MinimalGuildInfo, ReplyData};
use crate::error::ApiMessageError;
use std::sync::Arc;
use twilight_model::id::UserId;

pub async fn get_mutual_guilds(ctx: &Arc<BotContext>, user_id: &UserId) -> Result<ReplyData, ApiMessageError> {
    let guilds = ctx.cache.get_mutual_guilds(user_id).await;
    let mut out = vec![];
    for guild in guilds {
        out.push(MinimalGuildInfo {
            id: guild.id.0,
            name: guild.name.clone(),
            icon: guild.icon.clone(),
            owned: guild.owner_id == *user_id,
            permissions: ctx
                .get_permissions_for(
                    &guild,
                    &(guild.get_member(user_id).await).unwrap(),
                    &ctx.get_config(guild.id).await?,
                )
                .await,
        })
    }
    Ok(ReplyData::MutualGuildList(out))
}
