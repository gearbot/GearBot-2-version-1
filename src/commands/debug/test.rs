use crate::core::logpump::LogType;
use crate::core::CommandContext;
use crate::error::CommandResult;

pub async fn test(ctx: CommandContext) -> CommandResult {
    for _ in 1..5 {
        ctx.log(
            LogType::CommandUsed {
                command: "test".to_string(),
            },
            Some(ctx.message.channel.get_id()),
            ctx.message.author.id,
        );
    }

    Ok(())
}
