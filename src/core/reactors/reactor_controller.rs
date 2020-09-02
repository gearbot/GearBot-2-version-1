use std::sync::Arc;

use twilight::model::channel::Reaction;
// use twilight::model::id::{GuildId, UserId};

use crate::core::{BotContext, Reactor};
use crate::utils::Error;

pub async fn process_reaction(bot_context: &Arc<BotContext>, reaction: &Reaction) -> Result<(), Error> {
    if let Some(reactor) = bot_context
        .redis_cache
        .get::<Reactor>(&format!("reactor:{}", reaction.message_id))
        .await?
    {
        if let Some(emoji) = reactor.processes(reaction) {
            reactor.do_your_thing(emoji, bot_context, reaction).await?
        }
    }
    Ok(())
}
