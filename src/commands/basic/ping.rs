use std::time::{Duration, Instant};

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::translation::{FluArgs, GearBotString};

pub async fn ping(ctx: CommandContext) -> CommandResult {
    let start = Instant::now();

    let sent_msg = ctx.reply_raw(":ping_pong:").await?;

    let rest_time = start.elapsed().as_millis();

    let cluster_info = ctx.get_cluster_info();

    // This is 0 until we get a heartbeat
    let ws_time_avg = cluster_info
        .into_iter()
        .filter_map(|(_, info)| info.latency().average())
        .sum::<Duration>()
        .as_millis();

    let args = FluArgs::with_capacity(2)
        .add("rest", rest_time)
        .add("latency", ws_time_avg)
        .generate();

    let edited_msg = ctx.translate_with_args(GearBotString::PingPong, &args);

    ctx.update_message(edited_msg, sent_msg.channel_id, sent_msg.id).await?;

    Ok(())
}
