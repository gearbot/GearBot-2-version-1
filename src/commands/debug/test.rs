// use twilight_model::channel::{Reaction, ReactionType};
// use twilight_model::id::GuildId;

use crate::core::{CommandContext, LogType};
use crate::CommandResult;
use twilight_model::id::UserId;

pub async fn test(mut ctx: CommandContext) -> CommandResult {
    let arg = ctx.parser.get_remaining();
    ctx.log(LogType::TEST(arg), None, None);
    Ok(())
}
