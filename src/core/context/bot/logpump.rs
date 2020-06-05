use crate::core::BotContext;
use crate::utils::{Error, LogType};

use chrono::Utc;
use twilight::model::id::GuildId;

impl BotContext {
    pub fn log(&self, guild_id: GuildId, log: LogType) -> Result<(), Error> {
        match self.log_pumps.get(&guild_id) {
            Some(pump) => {
                pump.value()
                    .send((Utc::now(), log))
                    .map_err(|_| Error::LogError(guild_id))?;
                Ok(())
            }
            None => Ok(()),
        }
    }
}
