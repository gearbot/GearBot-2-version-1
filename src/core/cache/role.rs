use super::{get_true, is_default, is_true};
use dashmap::ElementGuard;
use serde::{Deserialize, Serialize};
use twilight::model::guild::{Permissions, Role};
use twilight::model::id::RoleId;

#[derive(Debug, Serialize, Deserialize)]
pub struct CachedRole {
    #[serde(rename = "a")]
    pub id: RoleId,
    #[serde(rename = "b")]
    pub name: String,
    #[serde(rename = "c", default, skip_serializing_if = "is_default")]
    pub color: u32,
    #[serde(rename = "d", default, skip_serializing_if = "is_default")]
    pub hoisted: bool,
    #[serde(rename = "e", default, skip_serializing_if = "is_default")]
    pub position: i64,
    #[serde(rename = "f")]
    pub permissions: Permissions,
    #[serde(rename = "g", default, skip_serializing_if = "is_default")]
    pub managed: bool,
    #[serde(rename = "h", default, skip_serializing_if = "is_default")]
    pub mentionable: bool,
}

impl From<Role> for CachedRole {
    fn from(role: Role) -> Self {
        CachedRole {
            id: role.id,
            name: role.name,
            color: role.color,
            hoisted: role.hoist,
            position: role.position,
            permissions: role.permissions,
            managed: role.managed,
            mentionable: role.mentionable,
        }
    }
}

impl From<ElementGuard<RoleId, CachedRole>> for CachedRole {
    fn from(guard: ElementGuard<RoleId, CachedRole>) -> Self {
        CachedRole {
            id: guard.id,
            name: guard.name.clone(),
            color: guard.color,
            hoisted: guard.hoisted,
            position: guard.position,
            permissions: guard.permissions,
            managed: guard.managed,
            mentionable: guard.mentionable,
        }
    }
}
