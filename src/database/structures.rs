use twilight::model::channel::message::MessageType;
use twilight::model::id::{ChannelId, GuildId, UserId};

#[derive(Debug)]
pub struct UserMessage {
    pub content: String,
    pub author: UserId,
    pub channel: ChannelId,
    pub guild: GuildId,
    pub kind: MessageType,
    pub pinned: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub(super) struct StoredUserMessage {
    pub id: i64,
    pub encrypted_content: Vec<u8>,
    pub author_id: i64,
    pub channel_id: i64,
    pub guild_id: i64,
    pub kind: i16,
    pub pinned: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StoredAttachment {
    id: i64,
    filename: String,
    is_image: bool,
    message_id: i64,
}

impl StoredUserMessage {
    pub fn kind(&self) -> MessageType {
        // TODO: This should exist in twilight via a TryFrom
        match self.kind as u8 {
            0 => MessageType::Regular,
            1 => MessageType::RecipientAdd,
            2 => MessageType::RecipientRemove,
            3 => MessageType::Call,
            4 => MessageType::ChannelNameChange,
            5 => MessageType::ChannelIconChange,
            6 => MessageType::ChannelMessagePinned,
            7 => MessageType::GuildMemberJoin,
            8 => MessageType::UserPremiumSub,
            9 => MessageType::UserPremiumSubTier1,
            10 => MessageType::UserPremiumSubTier2,
            11 => MessageType::UserPremiumSubTier3,
            12 => MessageType::ChannelFollowAdd,
            14 => MessageType::GuildDiscoveryDisqualified,
            15 => MessageType::GuildDiscoveryRequalified,
            _ => unreachable!(),
        }
    }
}
