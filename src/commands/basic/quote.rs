use crate::core::Context;
use crate::parser::Parser;
use crate::CommandResult;
use log::info;
use std::sync::Arc;
use twilight::model::channel::Message;

pub async fn quote(_ctx: Arc<Context>, msg: Message, mut parser: Parser) -> CommandResult {
    let message = parser.get_message(msg.author.id).await?;
    info!("{:?}", message);
    Ok(())
}
