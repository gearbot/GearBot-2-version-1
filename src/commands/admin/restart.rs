use crate::core::{BotContext, ColdRebootData, CommandContext};
use crate::gearbot_important;
use crate::parser::Parser;
use crate::CommandResult;
use log::info;
use std::collections::HashMap;
use std::sync::Arc;
use twilight::model::channel::Message;
use twilight::model::gateway::payload::UpdateStatus;
use twilight::model::gateway::presence::{Activity, ActivityType, Status};

pub async fn restart(ctx: CommandContext, msg: Message, _: Parser) -> CommandResult {
    if msg.author.id.0 == 106354106196570112 {
        ctx.send_message("Shutting down", msg.channel_id).await?;
        gearbot_important!("Reboot initiated by {}", msg.author.name);
        ctx.initiate_cold_resume().await?;
    }
    Ok(())
}
