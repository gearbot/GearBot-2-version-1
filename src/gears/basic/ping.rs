use std::sync::Arc;

use twilight::model::channel::Message;

use crate::core::Context;
use crate::CommandResult;

pub async fn ping(ctx: &Arc<Context<'_>>, msg: &Message) -> CommandResult {
    ctx.http
        .create_message(msg.channel_id)
        .content("Pong!")
        .await
        .unwrap();

    Ok(())
}
