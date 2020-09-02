use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight::model::channel::Reaction;
// use twilight::model::id::MessageId;

use crate::core::cache::CachedMember;
use crate::core::BotContext;
use crate::utils::{Emoji, Error};

#[derive(Deserialize, Serialize, Debug)]
pub struct EmojiListReactor {
    pub page: u8,
}

impl EmojiListReactor {
    pub fn processes(&self, reaction: &Reaction) -> Option<Emoji> {
        log::debug!("{:?}", reaction.emoji);
        for e in vec![Emoji::Left, Emoji::Right] {
            log::debug!("{:?}", e);

            if e.matches(&reaction.emoji) {
                return Some(e);
            }
        }
        None
    }

    pub async fn do_the_thing(
        &mut self,
        emoji: Emoji,
        ctx: &Arc<BotContext>,
        member: Option<Arc<CachedMember>>,
        reaction: &Reaction,
    ) -> Result<(), Error> {
        ctx.http
            .create_message(reaction.channel_id)
            .content("HI THERE")?
            .await?;
        self.page = 5;
        Ok(())
    }
}
