use std::sync::Arc;

use twilight::model::channel::Message;

use crate::CommandResult;
use crate::core::Context;

pub async fn echo(ctx: &Arc<Context<'_>>, msg: &Message, args: &str) -> CommandResult {
    ctx.http.create_message(msg.channel_id).content(args).await.unwrap();

    Ok(())
}
