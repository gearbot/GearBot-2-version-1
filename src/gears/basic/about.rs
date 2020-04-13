use std::sync::Arc;

use twilight::model::channel::Message;

use crate::CommandResult;
use crate::core::Context;

pub async fn about(ctx: &Arc<Context<'_>>, msg: &Message) -> CommandResult {
    ctx.http.create_message(msg.channel_id)
        .content("Still working on that...")
        .await.unwrap();
    
    Ok(())
}