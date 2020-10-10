use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ApiRequest {
    pub uuid: Uuid,
    pub request: Request,
}

#[derive(Debug, Deserialize)]
pub enum Request {
    TeamInfo,
}

impl Request {
    pub fn get_type(&self) -> &str {
        match self {
            Request::TeamInfo => "Team info",
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
    pub id: u64,
    pub team: String,
    pub socials: TeamSocials,
}

#[derive(Debug, Serialize)]
pub struct TeamMember {
    pub username: String,
    pub discriminator: String,
    pub id: u64,
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
