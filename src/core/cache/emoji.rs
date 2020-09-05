use serde::{Deserialize, Serialize};
use twilight::model::guild::Emoji;
use twilight::model::id::{EmojiId, RoleId, UserId};

use super::{get_true, is_default, is_true};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedEmoji {
    #[serde(rename = "a")]
    pub id: EmojiId,
    #[serde(rename = "b")]
    pub name: String,
    #[serde(rename = "c", default, skip_serializing_if = "is_default")]
    pub roles: Vec<RoleId>,
    #[serde(rename = "d", default, skip_serializing_if = "is_default")]
    pub created_by: Option<UserId>,
    #[serde(rename = "i", default, skip_serializing_if = "is_default")]
    pub requires_colons: bool,
    #[serde(rename = "j", default, skip_serializing_if = "is_default")]
    pub managed: bool,
    #[serde(rename = "k", default, skip_serializing_if = "is_default")]
    pub animated: bool,
    #[serde(rename = "l", default = "get_true", skip_serializing_if = "is_true")]
    pub available: bool,
}

impl CachedEmoji {
    pub fn get_url(&self) -> String {
        format!(
            "https://cdn.discordapp.com/emojis/{}.{}",
            self.id,
            if self.animated { "gif" } else { "png" }
        )
    }
}

impl From<Emoji> for CachedEmoji {
    fn from(emoji: Emoji) -> Self {
        let creator = match emoji.user {
            Some(e) => Some(e.id),
            None => None,
        };
        CachedEmoji {
            id: emoji.id,
            name: emoji.name,
            roles: emoji.roles,
            created_by: creator,
            requires_colons: emoji.require_colons,
            managed: emoji.managed,
            animated: emoji.animated,
            available: emoji.available,
        }
    }
}
