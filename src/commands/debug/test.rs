use crate::core::{CommandContext, LogType};
use crate::error::CommandResult;

pub async fn test(mut ctx: CommandContext) -> CommandResult {
    let arg = ctx.parser.get_remaining();
    ctx.log(LogType::TEST(arg), None, None);
    Ok(())
}
