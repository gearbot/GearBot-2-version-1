use crate::core::CommandContext;
use crate::error::CommandResult;

pub async fn help(ctx: CommandContext) -> CommandResult {
    match ctx.parser.peek() {
        Some(_) => {
            // user is asking about something
        }
        None => {
            // list everything
        }
    };
    Ok(())
}
