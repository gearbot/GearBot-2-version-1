use super::{get_true, is_default, is_true};
use crate::core::CachedUser;
use dashmap::ElementGuard;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use twilight::model::guild::Member;
use twilight::model::id::{RoleId, UserId};

#[derive(Debug)]
pub struct CachedMember {
    pub user: Arc<CachedUser>,
    pub nickname: Option<String>,
    pub roles: Vec<RoleId>,
    pub joined_at: Option<String>,
    //TODO: convert to date
    pub boosting_since: Option<String>,
    pub server_deafened: bool,
    pub server_muted: bool,
}

impl From<Member> for CachedMember {
    fn from(member: Member) -> Self {
        unimplemented!()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColdStorageMember {
    #[serde(rename = "i", default, skip_serializing_if = "is_default")]
    pub id: UserId,
    #[serde(rename = "n", default, skip_serializing_if = "is_default")]
    pub nickname: Option<String>,
    #[serde(rename = "r", default, skip_serializing_if = "is_default")]
    pub roles: Vec<RoleId>,
    #[serde(rename = "j", default, skip_serializing_if = "is_default")]
    pub joined_at: Option<String>,
    #[serde(rename = "b", default, skip_serializing_if = "is_default")]
    pub boosting_since: Option<String>,
    #[serde(rename = "d", default, skip_serializing_if = "is_default")]
    pub server_deafened: bool,
    #[serde(rename = "m", default, skip_serializing_if = "is_default")]
    pub server_muted: bool,
}

impl From<ElementGuard<UserId, Arc<CachedMember>>> for ColdStorageMember {
    fn from(member: ElementGuard<UserId, Arc<CachedMember>>) -> Self {
        ColdStorageMember {
            id: member.user.id,
            nickname: member.nickname.clone(),
            roles: member.roles.clone(),
            joined_at: member.joined_at.clone(),
            boosting_since: member.boosting_since.clone(),
            server_deafened: member.server_deafened,
            server_muted: member.server_muted,
        }
    }
}
