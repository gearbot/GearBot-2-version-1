use log::info;

use crate::core::CommandContext;
use crate::parser::Parser;
use crate::CommandResult;

pub async fn quote(ctx: CommandContext, mut parser: Parser) -> CommandResult {
    let message = parser.get_message(ctx.message.author.id).await?;
    info!("{:?}", message);
    Ok(())
}
