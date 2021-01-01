use crate::core::BotContext;
use crate::database::redis::api_handlers::api_structs::{ReplyData, UserInfo};
use crate::error::{ApiMessageError, ParseError};
use std::sync::Arc;
use twilight_model::id::UserId;

pub async fn get_user_info(ctx: &Arc<BotContext>, user: UserId) -> Result<ReplyData, ApiMessageError> {
    let info = match ctx.get_user(user).await {
        Ok(info) => Some(UserInfo {
            id: info.id.to_string(),
            name: info.username.clone(),
            discriminator: info.discriminator.clone(),
            avatar: info.avatar.clone(),
            bot_user: info.bot_user,
            system_user: info.system_user,
            public_flags: info.public_flags.clone(),
        }),
        Err(e) => match e {
            ParseError::InvalidUserID(_) => None,
            e => return Err(ApiMessageError::Parse(e)),
        },
    };
    Ok(ReplyData::UserInfo(info))
}
