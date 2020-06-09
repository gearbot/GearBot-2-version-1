use super::{get_true, is_default, is_true};
use serde::{Deserialize, Serialize};
use twilight::model::id::UserId;
use twilight::model::user::{User, UserFlags};

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedUser {
    #[serde(rename = "i")]
    pub id: UserId,
    #[serde(rename = "u")]
    pub username: String,
    #[serde(rename = "d")]
    pub discriminator: String,
    #[serde(rename = "a", default, skip_serializing_if = "is_default")]
    pub avatar: Option<String>,
    #[serde(rename = "b", default, skip_serializing_if = "is_default")]
    pub bot_user: bool,
    #[serde(rename = "s", default, skip_serializing_if = "is_default")]
    pub system_user: bool,
    #[serde(rename = "f", default, skip_serializing_if = "is_default")]
    pub public_flags: Option<UserFlags>,
}

impl From<User> for CachedUser {
    fn from(user: User) -> Self {
        CachedUser {
            id: user.id,
            username: user.name,
            discriminator: user.discriminator,
            avatar: user.avatar,
            bot_user: user.bot,
            system_user: user.system.unwrap_or(false),
            public_flags: user.public_flags,
        }
    }
}
