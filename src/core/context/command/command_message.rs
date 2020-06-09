use crate::core::{CachedChannel, CachedMember, CachedUser};
use std::sync::Arc;
use twilight::model::channel::embed::Embed;
use twilight::model::channel::message::{MessageFlags, MessageType};
use twilight::model::channel::{Attachment, ChannelMention};
use twilight::model::guild::Role;
use twilight::model::id::MessageId;
use twilight::model::user::User;

pub struct CommandMessage {
    pub id: MessageId,
    pub content: String,
    pub author: Arc<CachedUser>,
    pub author_as_member: Option<Arc<CachedMember>>,
    pub channel: Arc<CachedChannel>,
    pub attachments: Vec<Attachment>,
    pub embeds: Vec<Embed>,
    pub flags: Option<MessageFlags>,
    pub kind: MessageType,
    pub mention_everyone: bool,
    pub tts: bool,
}
