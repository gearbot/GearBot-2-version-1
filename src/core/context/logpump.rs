use crate::core::Context;
use crate::utils::LogType;
use chrono::Utc;

impl Context {
    pub fn log(&self, guild_id: u64, log: LogType) {
        match self.log_pumps.get(&guild_id) {
            Some(pump) => {
                pump.value().send((Utc::now(), log));
            }
            None => {}
        };
    }
}
