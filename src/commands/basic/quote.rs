use crate::core::CommandContext;
use crate::parser::Parser;
use crate::CommandResult;
use log::info;
use twilight::model::channel::Message;

pub async fn quote(_ctx: CommandContext, msg: Message, mut parser: Parser) -> CommandResult {
    let message = parser.get_message(msg.author.id).await?;
    info!("{:?}", message);
    Ok(())
}
