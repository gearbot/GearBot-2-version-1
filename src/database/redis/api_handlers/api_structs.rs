use crate::commands::meta::nodes::GearBotPermissions;
use serde::{Deserialize, Serialize};
use twilight_model::id::{GuildId, UserId};
use twilight_model::user::UserFlags;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub uuid: Uuid,
    pub request: Request,
}

#[derive(Debug, Deserialize)]
pub enum Request {
    TeamInfo,
    UserInfo(UserId),
    MutualGuilds(UserId),
}

impl Request {
    pub fn get_type(&self) -> &str {
        match self {
            Request::TeamInfo => "Team info",
            Request::UserInfo { .. } => "User info",
            Request::MutualGuilds(_) => "User mutual guilds",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Reply {
    pub uuid: Uuid,
    pub data: ReplyData,
}

#[derive(Debug, Serialize)]
pub enum ReplyData {
    TeamInfo(TeamInfo),
    UserInfo(Option<UserInfo>),
    MutualGuildList(Vec<MinimalGuildInfo>),
}

#[derive(Debug, Serialize)]
pub struct TeamInfo {
    pub members: Vec<TeamMember>,
}

#[derive(Debug, Deserialize)]
pub struct RawTeamMembers {
    pub members: Vec<RawTeamMember>,
}

#[derive(Debug, Deserialize)]
pub struct RawTeamMember {
    pub id: String,
    pub team: String,
    pub socials: TeamSocials,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub username: String,
    pub discriminator: String,
    pub id: String,
    pub avatar: String,
    pub team: String,
    pub socials: TeamSocials,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TeamSocials {
    pub twitter: Option<String>,
    pub github: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub discriminator: String,
    #[serde(skip_serializing_if = "is_default")]
    pub avatar: Option<String>,
    #[serde(skip_serializing_if = "is_default")]
    pub bot_user: bool,
    #[serde(skip_serializing_if = "is_default")]
    pub system_user: bool,
    #[serde(skip_serializing_if = "is_default")]
    pub public_flags: Option<UserFlags>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MinimalGuildInfo {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing_if = "is_default")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "is_default")]
    pub owned: bool,
    pub permissions: GearBotPermissions,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}
