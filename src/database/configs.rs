use log::info;

use super::{crypto, DataStorage};
use crate::core::GuildConfig;
use crate::error::DatabaseError;

impl DataStorage {
    /// Fetches a guild configuration from the database, returning it if it existed.
    ///
    /// The permissions inside the config are guaranteed to be in the correct order.
    pub async fn get_guild_config(&self, guild_id: u64) -> Result<Option<GuildConfig>, DatabaseError> {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT config from guildconfig where id=$1")
            .bind(guild_id as i64)
            .fetch_optional(&self.persistent_pool)
            .await?;

        let config = if let Some(c_val) = row {
            let mut config: GuildConfig = serde_json::from_value(c_val.0).map_err(DatabaseError::Deserializing)?;
            //CRITICAL: make sure permissions are propertly sorted
            config.permission_groups.sort_by(|a, b| a.priority.cmp(&b.priority));
            Some(config)
        } else {
            None
        };

        Ok(config)
    }

    /// Creates a new guild configuration for the specified guild and inserts it into the database.
    pub async fn create_new_guild_config(&self, guild_id: u64) -> Result<GuildConfig, DatabaseError> {
        info!("No config found for {}, inserting blank one", guild_id);
        let new_config = GuildConfig::default();

        let master_ek = &self.primary_encryption_key;
        let guild_encryption_key = crypto::generate_guild_encryption_key(master_ek, guild_id);

        sqlx::query("INSERT INTO guildconfig (id, config, encryption_key) VALUES ($1, $2, $3)")
            .bind(guild_id as i64)
            .bind(serde_json::to_value(&new_config).map_err(DatabaseError::Serializing)?)
            .bind(guild_encryption_key)
            .execute(&self.persistent_pool)
            .await?;

        Ok(new_config)
    }

    /// Updates a guild config for the specified guild with the provided new value.
    ///
    /// Errors if the guild doesn't exist already.
    pub async fn set_guild_config(&self, guild_id: u64, config: &GuildConfig) -> Result<(), DatabaseError> {
        sqlx::query("UPDATE guildconfig set config=$1 WHERE id=$2")
            .bind(serde_json::to_value(config).map_err(DatabaseError::Serializing)?)
            .bind(guild_id as i64)
            .execute(&self.persistent_pool)
            .await?;

        Ok(())
    }
}
