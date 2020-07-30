use log::info;

use crate::core::CommandContext;
use crate::CommandResult;

pub async fn quote(ctx: CommandContext) -> CommandResult {
    let message = ctx.message.content;

    info!("{:?}", message);
    Ok(())
}
