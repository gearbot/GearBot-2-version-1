use std::fs;

use serde::Deserialize;

use crate::utils::errors::Error;

#[derive(Deserialize, Debug)]
pub struct BotConfig {
    pub tokens: Tokens,
    pub logging: Logging,
    pub database: Database
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
    pub postgres: String
}

impl BotConfig {
    pub fn new(filename: &str) -> Result<Self, Error> {
        let config_file = fs::read_to_string(filename).map_err(|_| Error::NoConfig)?;
        match toml::from_str(&config_file) {
            Err(_) => Err(Error::InvalidConfig),
            Ok(c) => Ok(c),
        }
    }
}
