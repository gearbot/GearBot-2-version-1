use crate::utils::Error;
use deadpool_postgres::Pool;
use postgres_types::Type;
use twilight::model::channel::Message;

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
