use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU64;
use twilight_model::id::UserId;
use twilight_model::user::{User, UserFlags};

use super::is_default;

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
    #[serde(skip_serializing, default)]
    pub mutual_servers: AtomicU64,
}

impl Clone for CachedUser {
    fn clone(&self) -> Self {
        CachedUser {
            id: self.id,
            username: self.username.clone(),
            discriminator: self.discriminator.clone(),
            avatar: self.avatar.clone(),
            bot_user: self.bot_user,
            system_user: self.system_user,
            public_flags: self.public_flags,
            mutual_servers: AtomicU64::new(0),
        }
    }
}

impl CachedUser {
    pub(crate) fn from_user(user: &User) -> Self {
        CachedUser {
            id: user.id,
            username: user.name.clone(),
            discriminator: user.discriminator.clone(),
            avatar: user.avatar.clone(),
            bot_user: user.bot,
            system_user: user.system.unwrap_or(false),
            public_flags: user.public_flags,
            mutual_servers: AtomicU64::new(0),
        }
    }

    pub fn is_same_as(&self, user: &User) -> bool {
        self.id == user.id
            && self.username == user.name
            && self.discriminator == user.discriminator
            && self.avatar == user.avatar
            && self.bot_user == user.bot
            && self.system_user == user.system.unwrap_or(false)
            && self.public_flags == user.public_flags
    }

    pub fn full_name(&self) -> String {
        format!("{}#{}", self.username, self.discriminator)
    }

    pub fn full_name_with_id(&self) -> String {
        format!("{}#{} ({})", self.username, self.discriminator, self.id)
    }

    pub fn profile_link(&self) -> String {
        format!("https://discord.com/users/{}", self.id)
    }

    pub fn avatar_url(&self) -> String {
        match &self.avatar {
            Some(avatar) => format!(
                "https://cdn.discordapp.com/avatars/{}/{}.{}",
                self.id,
                avatar,
                if avatar.starts_with("a_") { "gif" } else { "png" }
            ),
            None => format!(
                "https://cdn.discordapp.com/embed/avatar/{}.png",
                &self.discriminator.parse::<u16>().unwrap() % 5
            ),
        }
    }
}
