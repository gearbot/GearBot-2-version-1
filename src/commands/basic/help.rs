use crate::core::CommandContext;
use crate::error::CommandResult;

pub async fn help(ctx: CommandContext) -> CommandResult {
    if let Some(_thing) = ctx.parser.peek() {
        // user is asking about something
    } else {
        // list everything
    }

    Ok(())
}
