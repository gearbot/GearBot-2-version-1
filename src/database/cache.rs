use deadpool_postgres::Pool;
use postgres_types::Type;
use twilight::model::channel::message::MessageType;
use twilight::model::channel::{Attachment, Message};

use crate::utils::Error;

pub async fn insert_message(pool: &Pool, content: Vec<u8>, msg: &Message) -> Result<(), Error> {
    let client = pool.get().await?;
    let statement = client
        .prepare_typed(
            "INSERT INTO message (id, content, author_id, channel_id, guild_id, type, pinned)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[Type::INT8, Type::BYTEA, Type::INT8, Type::INT8, Type::INT8, Type::INT2, Type::BOOL],
        )
        .await?;

    client
        .execute(
            &statement,
            &[
                &(msg.id.0 as i64),
                &content,
                &(msg.author.id.0 as i64),
                &(msg.channel_id.0 as i64),
                &(msg.guild_id.unwrap().0 as i64),
                &(msg.kind.clone() as i16),
                &msg.pinned,
            ],
        )
        .await?;
    Ok(())
}

pub async fn insert_attachment(pool: &Pool, message_id: u64, attachment: &Attachment) -> Result<(), Error> {
    let client = pool.get().await?;
    let statement = client
        .prepare_typed(
            "INSERT INTO attachment (id, name, image, message_id)
        VALUES ($1, $2, $3, $4);",
            &[Type::INT8, Type::VARCHAR, Type::BOOL, Type::INT8],
        )
        .await?;
    client
        .execute(
            &statement,
            &[
                &(attachment.id.0 as i64),
                &attachment.filename,
                &attachment.width.is_some(),
                &(message_id as i64),
            ],
        )
        .await?;
    Ok(())
}

pub async fn get_full_message(pool: &Pool, message_id: u64) -> Result<Option<(Vec<u8>, u64, u64, u64, MessageType, bool)>, Error> {
    let client = pool.get().await?;

    let statement = client.prepare_typed("SELECT * from message where id=$1", &[Type::INT8]).await?;

    let fetch_id = message_id as i64;

    let rows = client.query(&statement, &[&fetch_id]).await?;
    if let Some(stored_msg) = rows.get(0) {
        let encrypted_message: &[u8] = stored_msg.get(1);
        let author: i64 = stored_msg.get(2);
        let channel: i64 = stored_msg.get(3);
        let guild_id: i64 = stored_msg.get(4);

        let raw_msg_type: i16 = stored_msg.get(5);
        let pinned: bool = stored_msg.get(6);

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
            _ => unimplemented!(),
        };

        Ok(Some((
            encrypted_message.to_owned(),
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

pub async fn get_channel_for_message(pool: &Pool, msg_id: u64) -> Result<Option<u64>, Error> {
    let client = pool.get().await?;

    let statement = client.prepare_typed("SELECT channel_id from message where id=$1", &[Type::INT8]).await?;

    let rows = client.query(&statement, &[&(msg_id as i64)]).await?;
    if let Some(stored_msg) = rows.get(0) {
        let channel_id: i64 = stored_msg.get(0);
        Ok(Some(channel_id as u64))
    } else {
        Ok(None)
    }
}
