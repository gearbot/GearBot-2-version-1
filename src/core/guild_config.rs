use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GuildConfig {
    pub(crate) prefix: String,
}

impl Default for GuildConfig {
    fn default() -> Self {
        GuildConfig {
            prefix: "!".to_string(),
        }
    }
}
