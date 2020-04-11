use crate::{CommandResult, Context};
use twilight::model::channel::Message;
use std::sync::Arc;

pub async fn about(ctx: &Arc<Context<'_>>, msg: &Message) -> CommandResult {
    ctx.http.create_message(msg.channel_id)
        .content("Still working on that...")
        .await.unwrap();
    
    Ok(())
}