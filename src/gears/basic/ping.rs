use crate::{CommandResult, Context};
use twilight::model::channel::Message;
use std::sync::Arc;

pub async fn ping(ctx: &Arc<Context<'_>>, msg: &Message) -> CommandResult {
    ctx.http.create_message(msg.channel_id)
        .content("Pong!")
        .await.unwrap();
    
    Ok(())
}