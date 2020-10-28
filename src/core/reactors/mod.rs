use std::sync::Arc;

use serde::{Deserialize, Serialize};
use twilight_model::channel::Reaction;
use twilight_model::id::MessageId;

use crate::core::bot_context::BotContext;
use crate::core::reactors::emoji_list_reactor::EmojiListReactor;
use crate::error::{DatabaseError, ReactorError};
use crate::utils::Emoji;

mod emoji_list_reactor;
mod help_reactor;
pub mod reactor_controller;

pub use emoji_list_reactor::gen_emoji_page;

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
        self,
        emoji: Emoji,
        ctx: &Arc<BotContext>,
        reaction: &Reaction,
    ) -> Result<Self, ReactorError> {
        let member = match &reaction.guild_id {
            Some(guild_id) => ctx.cache.get_member(guild_id, &reaction.user_id),
            None => None,
        };
        let new = match self {
            Reactor::Help => self,
            Reactor::EmojiList(mut inner) => {
                inner.do_the_thing(emoji.clone(), ctx, member, reaction).await?;
                Reactor::EmojiList { 0: inner }
            }
        };

        new.save(ctx, reaction.message_id).await?;
        ctx.http
            .delete_reaction(
                reaction.channel_id,
                reaction.message_id,
                emoji.to_reaction(),
                reaction.user_id.clone(),
            )
            .await?;
        Ok(new)
    }

    pub async fn save(&self, ctx: &Arc<BotContext>, message_id: MessageId) -> Result<(), DatabaseError> {
        ctx.datastore
            .cache_pool
            .set(&format!("reactor:{}", message_id), self, Some(self.get_expiry()))
            .await
    }

    fn get_expiry(&self) -> u32 {
        match self {
            _ => 60 * 60 * 24,
        }
    }
}

pub fn get_emoji(options: Vec<Emoji>, reaction: &Reaction) -> Option<Emoji> {
    for e in options {
        if e.matches(&reaction.emoji) {
            return Some(e);
        }
    }
    None
}

pub fn scroll_page(pages: u8, current: u8, emoji: &Emoji) -> u8 {
    match emoji {
        Emoji::Left => {
            if current == 0 {
                pages - 1
            } else {
                current - 1
            }
        }
        Emoji::Right => {
            if current + 1 == pages {
                0
            } else {
                current + 1
            }
        }
        _ => current,
    }
}
