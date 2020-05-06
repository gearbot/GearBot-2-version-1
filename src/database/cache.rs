use crate::utils::Error;
use deadpool_postgres::Pool;
use postgres_types::Type;
use twilight::model::channel::{Attachment, Message};

pub async fn insert_message(pool: &Pool, content: Vec<u8>, msg: &Message) -> Result<(), Error> {
    let client = pool.get().await?;
    let statement = client
        .prepare_typed(
            "INSERT INTO message (id, content, author_id, channel_id, guild_id, type, pinned)
            VALUES ($1, $2, $3, $4, $5, $6, $7)",
            &[
                Type::INT8,
                Type::BYTEA,
                Type::INT8,
                Type::INT8,
                Type::INT8,
                Type::INT2,
                Type::BOOL,
            ],
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
                &(msg.kind as i16),
                &msg.pinned,
            ],
        )
        .await?;
    Ok(())
}

pub async fn insert_attachment(
    pool: &Pool,
    message_id: u64,
    attachment: &Attachment,
) -> Result<(), Error> {
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
