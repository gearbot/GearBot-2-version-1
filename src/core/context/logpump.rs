use crate::core::Context;
use crate::utils::Error;
use crate::utils::LogType;
use chrono::Utc;

impl Context {
    pub fn log(&self, guild_id: u64, log: LogType) -> Result<(), Error> {
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
