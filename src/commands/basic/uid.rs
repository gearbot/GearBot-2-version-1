use std::sync::Arc;

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;
use crate::utils::{matchers, ParseError};

pub async fn uid(ctx: Arc<Context>, msg: Message, mut parser: Parser) -> CommandResult {
    let user_id = {
        let msg = parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    }; 
    
    ctx.http.create_message(msg.channel_id)
        .content(user_id.to_string())
        .await?;

    Ok(())
}