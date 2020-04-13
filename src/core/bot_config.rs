use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct BotConfig {
    pub tokens: Tokens,
    pub logging: Logging
}

#[derive(Deserialize, Debug)]
pub struct Tokens {
    pub discord: String
}

#[derive(Deserialize, Debug)]
pub struct Logging {
    pub important_logs: String,
    pub info_logs: String
}

impl BotConfig {
    pub fn new(filename: &str) -> Result<Self, String> {
        match fs::read_to_string(filename) {
            Err(_e) => Err(String::from("Failed to open config file")),
            Ok(content) => {
                match toml::from_str(content.as_str()) {
                    Err(e) => Err(e.to_string()),
                    Ok(c) => Ok(c)
                }
            }
        }

    }
}
