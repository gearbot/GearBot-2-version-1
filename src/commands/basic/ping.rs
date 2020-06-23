use std::time::{Duration, Instant};

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::translation::{FluArgs, GearBotString};

pub async fn ping(ctx: CommandContext, _: Parser) -> CommandResult {
    let start = Instant::now();

    let sent_msg = ctx.reply_raw(":ping_pong:").await?;

    let finished = Instant::now();

    let rest_time = (finished - start).as_millis();

    let cluster_info = ctx.get_cluster_info().await;

    // This is 0 until we get a heartbeat
    let ws_time_avg = cluster_info
        .into_iter()
        .filter_map(|(_, info)| info.latency().average())
        .sum::<Duration>()
        .as_millis();

    let args = FluArgs::with_capacity(2)
        .insert("rest", rest_time)
        .insert("latency", ws_time_avg)
        .generate();

    let edited_msg = ctx.translate_with_args(GearBotString::PingPong, &args);

    ctx.update_message(edited_msg, sent_msg.channel_id, sent_msg.id).await?;

    Ok(())
}
