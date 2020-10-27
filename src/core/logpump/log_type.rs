use crate::core::guild_config::LogCategory;
use crate::error::MessageError;
use serde::{Deserialize, Serialize};
use twilight_embed_builder::{EmbedBuildError, EmbedBuilder};
use twilight_model::channel::embed::Embed;

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum LogType {
    TEST(String),
}

#[derive(Deserialize, Serialize, Debug, Eq, PartialEq)]
pub enum DataLessLogType {
    TEST,
}

impl LogType {
    pub fn get_category(&self) -> LogCategory {
        match self {
            LogType::TEST(_) => LogCategory::TEST,
        }
    }

    pub fn to_embed(&self) -> Result<Embed, MessageError> {
        Ok(match self {
            LogType::TEST(data) => EmbedBuilder::new().description(data)?,
        }
        .build()?)
    }

    pub fn to_text(&self) -> &str {
        match self {
            LogType::TEST(data) => data,
        }
    }

    pub fn dataless(&self) -> DataLessLogType {
        match self {
            Self::TEST(_) => DataLessLogType::TEST,
        }
    }
}
