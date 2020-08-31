use std::sync::Arc;

use twilight::model::channel::Reaction;
use twilight::model::id::{GuildId, UserId};

use crate::core::{BotContext, Reactor};
use crate::utils::Error;

pub async fn process_reaction(bot_context: &Arc<BotContext>, reaction: &Reaction) -> Result<(), Error> {
    let mut connection = bot_context.redis_pool.get().await;
    if let Some(content) = connection.get(format!("reactor:{}", reaction.message_id)).await? {
        let reactor: Reactor = serde_json::from_str(&*String::from_utf8(content).unwrap())?;
        if let Some(emoji) = reactor.processes(reaction) {
            reactor.do_your_thing(emoji, bot_context, reaction).await?
        }
    }
    Ok(())
}
