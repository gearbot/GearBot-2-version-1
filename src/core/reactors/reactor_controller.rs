use std::sync::Arc;

use twilight_model::channel::Reaction;

use crate::core::bot_context::BotContext;
use crate::core::Reactor;
use crate::error::ReactorError;

pub async fn process_reaction(bot_context: &Arc<BotContext>, reaction: &Reaction) -> Result<(), ReactorError> {
    if reaction.user_id != bot_context.bot_user.id {
        if let Some(reactor) = bot_context
            .datastore
            .cache_pool
            .get::<Reactor>(&format!("reactor:{}", reaction.message_id))
            .await?
        {
            if let Some(emoji) = reactor.processes(reaction) {
                reactor.do_your_thing(emoji, bot_context, reaction).await?;
            }
        }
    }
    Ok(())
}
