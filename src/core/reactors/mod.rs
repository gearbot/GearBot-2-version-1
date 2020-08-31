use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight::model::channel::{Reaction, ReactionType};
use twilight::model::id::MessageId;

use crate::core::cache::CachedMember;
use crate::core::reactors::emoji_list_reactor::EmojiListReactor;
use crate::core::BotContext;
use crate::utils::{Emoji, Error};

mod emoji_list_reactor;
mod help_reactor;
pub mod reactor_controller;

#[derive(Deserialize, Serialize, Debug)]
pub enum Reactor {
    Help,
    EmojiList(EmojiListReactor),
}

impl Reactor {
    pub fn new_emoji_list() -> Self {
        Reactor::EmojiList {
            0: EmojiListReactor { page: 0 },
        }
    }

    pub fn processes(&self, reaction: &Reaction) -> Option<Emoji> {
        match self {
            Reactor::Help => None,
            Reactor::EmojiList(inner) => inner.processes(reaction),
        }
    }

    pub async fn do_your_thing(
        mut self,
        emoji: Emoji,
        ctx: &Arc<BotContext>,
        reaction: &Reaction,
    ) -> Result<(), Error> {
        let member = match &reaction.guild_id {
            Some(guild_id) => ctx.cache.get_member(guild_id, &reaction.user_id),
            None => None,
        };
        match self {
            Reactor::Help => {}
            Reactor::EmojiList(mut inner) => {
                inner.do_the_thing(emoji, ctx, member, reaction).await?;
                log::info!("inner page count is now at {}", inner.page);
            }
        }
        Ok(())
    }

    pub async fn save(&self, ctx: &Arc<BotContext>, message_id: MessageId) -> Result<(), Error> {
        let mut connection = ctx.redis_pool.get().await;
        let serialized = serde_json::to_string(self).unwrap();
        connection
            .set_and_expire_seconds(format!("reactor:{}", message_id), serialized, self.get_expiry())
            .await?;
        Ok(())
    }

    fn get_expiry(&self) -> u32 {
        match self {
            _ => 60 * 60 * 24,
        }
    }
}
