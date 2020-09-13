use std::sync::Arc;

use twilight_model::id::UserId;

use crate::core::cache::CachedUser;
use crate::core::BotContext;
use crate::utils::{Error, ParseError};

impl BotContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<CachedUser>, Error> {
        match self.cache.get_user(user_id) {
            Some(user) => Ok(user),
            None => {
                // let's see if we can get em from the api
                let user = self.http.user(user_id).await?;
                //TODO: cache in redis

                match user {
                    Some(_) => Err(Error::ParseError(ParseError::InvalidUserID(user_id.0))), //Ok(user),
                    None => Err(Error::ParseError(ParseError::InvalidUserID(user_id.0))),
                }
            }
        }
    }
}
