use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::GuildContext;
use crate::parser::Parser;
use crate::utils::{matchers, ParseError};

pub async fn uid(ctx: GuildContext, msg: Message, mut parser: Parser) -> CommandResult {
    let user_id = {
        let msg = parser.get_next()?;
        matchers::get_mention(msg).ok_or(ParseError::MissingArgument)?
    };

    ctx.send_message(user_id.to_string(), msg.channel_id)
        .await?;

    Ok(())
}
