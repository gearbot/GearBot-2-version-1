use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight_model::guild::Member;
use twilight_model::id::{RoleId, UserId};

use super::{is_default, Cache, CachedUser};
use twilight_model::gateway::payload::MemberUpdate;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedMember {
    #[serde(rename = "i", default, skip_serializing_if = "is_default")]
    pub user_id: UserId,
    #[serde(rename = "n", default, skip_serializing_if = "is_default")]
    pub nickname: Option<String>,
    #[serde(rename = "r", default, skip_serializing_if = "is_default")]
    pub roles: Vec<RoleId>,
    #[serde(rename = "j", default, skip_serializing_if = "is_default")]
    pub joined_at: Option<String>,
    //TODO: convert to date
    #[serde(rename = "b", default, skip_serializing_if = "is_default")]
    pub boosting_since: Option<String>,
    #[serde(rename = "d", default, skip_serializing_if = "is_default")]
    pub server_deafened: bool,
    #[serde(rename = "m", default, skip_serializing_if = "is_default")]
    pub server_muted: bool,
}

impl CachedMember {
    pub fn from_member(member: &Member) -> Self {
        CachedMember {
            user_id: member.user.id,
            nickname: member.nick.clone(),
            roles: member.roles.clone(),
            joined_at: member.joined_at.clone(),
            boosting_since: member.premium_since.clone(),
            server_deafened: member.deaf,
            server_muted: member.mute,
        }
    }

    pub fn update(&self, member: &MemberUpdate) -> Self {
        CachedMember {
            user_id: member.user.id,
            nickname: member.nick.clone(),
            roles: member.roles.clone(),
            joined_at: self.joined_at.clone(),
            boosting_since: member.premium_since.clone(),
            server_deafened: self.server_deafened,
            server_muted: self.server_muted,
        }
    }

    pub fn user(&self, cache: &Cache) -> Arc<CachedUser> {
        cache
            .get_user(self.user_id)
            .expect("User got nuked from the global cache too early!")
    }

    pub fn duplicate(&self) -> Self {
        CachedMember {
            user_id: self.user_id,
            nickname: self.nickname.clone(),
            roles: self.roles.clone(),
            joined_at: self.joined_at.clone(),
            boosting_since: self.boosting_since.clone(),
            server_deafened: self.server_deafened,
            server_muted: self.server_muted,
        }
    }
}
