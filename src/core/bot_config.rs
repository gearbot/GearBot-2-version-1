use std::collections::HashMap;
use std::fs;

use serde::Deserialize;
use twilight_model::id::EmojiId;

use crate::error::StartupError;
use crate::utils::{emoji, matchers, EmojiOverride};

#[derive(Deserialize, Debug)]
pub struct BotConfig {
    pub main_encryption_key: Vec<u8>,
    pub tokens: Tokens,
    pub logging: Logging,
    pub database: Database,
    pub emoji: HashMap<String, String>,
    pub global_admins: Vec<u64>,
    pub proxy_url: Option<String>,
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
    pub redis: String,
}

impl BotConfig {
    pub fn new(filename: &str) -> Result<Self, StartupError> {
        let config_file = fs::read_to_string(filename).map_err(|_| StartupError::NoConfig)?;
        match toml::from_str::<BotConfig>(&config_file) {
            Err(_) => Err(StartupError::InvalidConfig),
            Ok(c) => {
                let mut override_map: HashMap<String, EmojiOverride> = HashMap::with_capacity(c.emoji.len());
                let mut id_map: HashMap<String, u64> = HashMap::with_capacity(c.emoji.len());

                for (name, value) in &c.emoji {
                    let info = matchers::get_emoji_parts(&value);

                    if info.len() != 1 {
                        panic!("Invalid emoji override found for {}", name)
                    }

                    let info = info.first().unwrap();

                    let id = matchers::get_emoji_parts(&value)[0].id;
                    let e_name = matchers::get_emoji_parts(&value)[0].name.clone();

                    override_map.insert(
                        name.clone(),
                        EmojiOverride {
                            id: EmojiId(info.id),
                            for_chat: value.clone(),
                            name: e_name,
                        },
                    );

                    id_map.insert(name.clone(), id);
                }
                emoji::EMOJI_OVERRIDES.set(override_map).unwrap();
                Ok(c)
            }
        }
    }
}
