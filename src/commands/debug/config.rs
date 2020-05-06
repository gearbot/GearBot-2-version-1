use crate::core::{Context, GuildConfig};
use crate::parser::Parser;
use crate::CommandResult;
use std::sync::Arc;
use twilight::model::channel::Message;

pub async fn get_config(ctx: Arc<Context>, msg: Message, _: Parser) -> CommandResult {
    ctx.http
        .create_message(msg.channel_id)
        .content(serde_json::to_string(
            ctx.get_config(msg.guild_id.unwrap()).await?.value(),
        )?)
        .await?;
    Ok(())
}

pub async fn set_config(ctx: Arc<Context>, msg: Message, mut parser: Parser) -> CommandResult {
    let config: GuildConfig = serde_json::from_str(&*parser.get_remaining())?;
    ctx.set_config(msg.guild_id.unwrap(), config).await?;
    Ok(())
}
