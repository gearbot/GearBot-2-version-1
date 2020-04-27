use std::collections::HashMap;
use std::fs;

use serde::Deserialize;

use crate::utils::{emoji, Error, matchers};
use crate::utils::Emoji;

#[derive(Deserialize, Debug)]
pub struct BotConfig {
    pub tokens: Tokens,
    pub logging: Logging,
    pub database: Database,

    pub emoji: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
pub struct Tokens {
    pub discord: String,
}

#[derive(Deserialize, Debug)]
pub struct Logging {
    pub important_logs: String,
    pub info_logs: String,
}

#[derive(Deserialize, Debug)]
pub struct Database {
    pub postgres: String,
}

impl BotConfig {
    pub fn new(filename: &str) -> Result<Self, Error> {
        let config_file = fs::read_to_string(filename).map_err(|_| Error::NoConfig)?;
        match toml::from_str::<BotConfig>(&config_file) {
            Err(_) => Err(Error::InvalidConfig),
            Ok(c) => {
                let mut override_map: HashMap<String, String> = HashMap::new();
                let mut id_map: HashMap<String, u64> = HashMap::new();
                for (name, value) in c.emoji.iter() {
                    override_map.insert(name.clone(), value.clone());
                    let id: u64 = matchers::get_emoji_parts(value)[0].id;
                    id_map.insert(name.clone(), id);
                }
                emoji::EMOJI_OVERRIDES.set(override_map);
                Ok(c)
            }
        }
    }
}
