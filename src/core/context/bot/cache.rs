use std::sync::Arc;

use twilight_model::id::UserId;

use crate::core::cache::CachedUser;
use crate::core::BotContext;
use crate::error::ParseError;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum UserHolder {
    Valid(CachedUser),
    Invalid,
}

const USER_CACHE_DURATION: u32 = 3600;

impl BotContext {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<CachedUser>, ParseError> {
        match self.cache.get_user(user_id) {
            Some(user) => Ok(user),
            None => {
                //try to find them in redis
                let redis_key = format!("user:{}", user_id);
                match self.redis_cache.get::<UserHolder>(&redis_key).await? {
                    Some(option) => match option {
                        UserHolder::Valid(user) => Ok(Arc::new(user)),
                        UserHolder::Invalid => Err(ParseError::InvalidUserID(user_id.0)),
                    },
                    None => {
                        // let's see if we can get em from the api
                        let user = self.http.user(user_id).await?;

                        match user {
                            Some(user) => {
                                let user = CachedUser::from_user(&user);
                                self.redis_cache
                                    .set(
                                        &redis_key,
                                        &UserHolder::Valid { 0: user.clone() },
                                        Some(USER_CACHE_DURATION),
                                    )
                                    .await?;
                                Ok(Arc::new(user))
                            }
                            None => {
                                self.redis_cache
                                    .set(&redis_key, &UserHolder::Invalid, Some(USER_CACHE_DURATION))
                                    .await?;
                                Err(ParseError::InvalidUserID(user_id.0))
                            }
                        }
                    }
                }
            }
        }
    }
}
