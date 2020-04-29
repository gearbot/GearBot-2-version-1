use postgres_types::ToSql;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct GuildConfig {
    pub(crate) prefix: String,
}

impl GuildConfig {
    pub fn new() -> Self {
        GuildConfig {
            prefix: String::from("!"),
        }
    }
}
