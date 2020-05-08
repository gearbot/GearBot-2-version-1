use crate::core::Context;
use crate::parser::Parser;
use crate::CommandResult;
use std::sync::Arc;
use twilight::model::channel::Message;

pub async fn quote(ctx: Arc<Context>, msg: Message, parser: Parser) -> CommandResult {
    Ok(())
}
