use crate::core::Context;
use crate::utils::ParseError::MemberNotFoundById;
use crate::utils::{Error, ParseError};
use futures::channel::oneshot;
use log::debug;
use std::sync::Arc;
use twilight::http::error::Error::Response;
use twilight::http::error::ResponseError::{Client, Server};
use twilight::http::error::{Error as HttpError, ResponseError};
use twilight::model::gateway::payload::{MemberChunk, RequestGuildMembers};
use twilight::model::gateway::presence::Presence;
use twilight::model::guild::Member;
use twilight::model::id::UserId;
use twilight::model::user::User;
use uuid::Uuid;

impl Context {
    pub async fn get_user(&self, user_id: UserId) -> Result<Arc<User>, Error> {
        match self.cache.user(user_id).await? {
            Some(user) => Ok(user),
            None => {
                // let's see if we can get em from the api
                let result = self.http.user(user_id.0).await;
                //TODO: cache in redis

                match result {
                    Ok(u) => {
                        let user = u.unwrap(); // there isn't a codepath that can even give none for this atm
                        Ok(Arc::new(user))
                    }
                    Err(error) => {
                        //2 options here:
                        //1) drill down 3 layers and get a headache trying to deal with moving and re-assembling errors to figure out the status code
                        //2) just get the string and find the code in there
                        if format!("{:?}", error).contains("status: 404") {
                            Err(Error::ParseError(ParseError::InvalidUserID(user_id.0)))
                        } else {
                            Err(Error::TwilightHttp(error))
                        }
                    }
                }
            }
        }
    }
}
