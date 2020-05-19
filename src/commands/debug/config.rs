use crate::core::{GuildConfig, GuildContext};
use crate::parser::Parser;
use crate::CommandResult;
use twilight::model::channel::Message;

pub async fn get_config(ctx: GuildContext, msg: Message, _: Parser) -> CommandResult {
    let stringified_config = serde_json::to_string(ctx.get_config()).unwrap();

    ctx.send_message(stringified_config, msg.channel_id).await?;

    Ok(())
}

pub async fn set_config(ctx: GuildContext, _: Message, mut parser: Parser) -> CommandResult {
    let config: GuildConfig = serde_json::from_str(&*parser.get_remaining())?;
    ctx.set_config(config).await?;
    Ok(())
}
