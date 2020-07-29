use twilight::model::channel::message::MessageType;
use twilight::model::channel::{Attachment, Message};

use crate::utils::Error;

pub async fn insert_message(pool: &sqlx::PgPool, content: Vec<u8>, msg: &Message) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO message (id, content, author_id, channel_id, guild_id, type, pinned)
        VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(msg.id.0 as i64)
    .bind(&content)
    .bind(msg.author.id.0 as i64)
    .bind(msg.channel_id.0 as i64)
    .bind(msg.guild_id.unwrap().0 as i64)
    .bind(msg.kind.clone() as i16)
    .bind(msg.pinned)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_attachment(pool: &sqlx::PgPool, message_id: u64, attachment: &Attachment) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO attachment (id, name, image, message_id)
        VALUES ($1, $2, $3, $4)",
    )
    .bind(attachment.id.0 as i64)
    .bind(&attachment.filename)
    .bind(attachment.width.is_some())
    .bind(message_id as i64)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_full_message(
    pool: &sqlx::PgPool,
    message_id: u64,
) -> Result<Option<(Vec<u8>, u64, u64, u64, MessageType, bool)>, Error> {
    let stored_message: Option<((Vec<u8>, i64, i64, i64, i16, bool),)> =
        sqlx::query_as("SELECT * from message where id=$1")
            .bind(message_id as i64)
            .fetch_optional(pool)
            .await?;

    if let Some(stored_msg) = stored_message {
        let stored_msg = stored_msg.0;

        let encrypted_message = stored_msg.0;
        let author: i64 = stored_msg.1;
        let channel: i64 = stored_msg.2;
        let guild_id: i64 = stored_msg.3;

        let raw_msg_type: i16 = stored_msg.4;
        let pinned: bool = stored_msg.5;

        // TODO: This should exist in twilight via a TryFrom
        let msg_type = match raw_msg_type as u8 {
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
        };

        Ok(Some((
            encrypted_message,
            author as u64,
            channel as u64,
            guild_id as u64,
            msg_type,
            pinned,
        )))
    } else {
        Ok(None)
    }
}

pub async fn get_channel_for_message(pool: &sqlx::PgPool, msg_id: u64) -> Result<Option<u64>, Error> {
    let row: Option<(i64,)> = sqlx::query_as("SELECT channel_id from message where id=$1")
        .bind(msg_id as i64)
        .fetch_optional(pool)
        .await?;

    let channel_id = if let Some(channel_id) = row {
        Some(channel_id.0 as u64)
    } else {
        None
    };

    Ok(channel_id)
}
