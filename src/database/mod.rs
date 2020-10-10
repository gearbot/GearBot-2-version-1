pub mod configs;
mod redis;
pub use redis::api_structs;
pub use redis::Redis;

pub mod structures;

use structures::{StoredUserMessage, UserMessage};

use twilight_model::channel::{Attachment, Message};
use twilight_model::id::{ChannelId, GuildId, MessageId, UserId};

use log::info;

use crate::crypto::{self, EncryptionKey};
use crate::utils::Error;

pub async fn insert_message(pool: &sqlx::PgPool, msg: &Message, guild_key: &EncryptionKey) -> Result<(), Error> {
    let start = std::time::Instant::now();

    let ciphertext = {
        let plaintext = msg.content.as_bytes();

        crypto::encrypt_bytes(plaintext, &guild_key, msg.id.0)
    };

    info!("It took {}us to encrypt the user message!", start.elapsed().as_micros());

    sqlx::query(
        "INSERT INTO message (id, encrypted_content, author_id, channel_id, guild_id, kind, pinned)
        VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(msg.id.0 as i64)
    .bind(ciphertext)
    .bind(msg.author.id.0 as i64)
    .bind(msg.channel_id.0 as i64)
    .bind(msg.guild_id.unwrap().0 as i64)
    .bind(msg.kind.clone() as i16)
    .bind(msg.pinned)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_attachment(
    pool: &sqlx::PgPool,
    message_id: MessageId,
    attachment: &Attachment,
) -> Result<(), Error> {
    sqlx::query(
        "INSERT INTO attachment (id, name, image, message_id)
        VALUES ($1, $2, $3, $4)",
    )
    .bind(attachment.id.0 as i64)
    .bind(&attachment.filename)
    .bind(attachment.width.is_some())
    .bind(message_id.0 as i64)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_full_message(
    pool: &sqlx::PgPool,
    message_id: MessageId,
    guild_key: &EncryptionKey,
) -> Result<Option<UserMessage>, Error> {
    let stored_message: Option<StoredUserMessage> = sqlx::query_as("SELECT * from message where id=$1")
        .bind(message_id.0 as i64)
        .fetch_optional(pool)
        .await?;

    let user_msg = match stored_message {
        Some(sm) => {
            let start = std::time::Instant::now();

            let guild_id = sm.guild_id as u64;
            let decrypted_content = crypto::decrypt_bytes(&sm.encrypted_content, &guild_key, message_id.0);

            info!("It took {}us to decrypt a user message!", start.elapsed().as_micros());

            Some(UserMessage {
                content: String::from_utf8(decrypted_content).unwrap(),
                author: UserId(sm.author_id as u64),
                channel: ChannelId(sm.channel_id as u64),
                guild: GuildId(guild_id),
                kind: sm.kind(),
                pinned: sm.pinned,
            })
        }
        None => None,
    };

    Ok(user_msg)
}
