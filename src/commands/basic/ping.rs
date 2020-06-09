use std::time::{Duration, Instant};

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::parser::Parser;
use crate::translation::BasicStrings;

pub async fn ping(ctx: CommandContext, _: Parser) -> CommandResult {
    let start = Instant::now();

    let sent_msg = ctx.reply(":ping_pong:").await?;

    let finished = Instant::now();

    let rest_time = (finished - start).as_millis();

    let cluster_info = ctx.get_cluster_info().await;

    // This is 0 until we get a heartbeat
    let ws_time_avg = cluster_info
        .into_iter()
        .filter_map(|(_, info)| info.latency().average())
        .sum::<Duration>()
        .as_millis();

    let arg_parts = [("rest", &rest_time), ("latency", &ws_time_avg)];
    let args = ctx.generate_args(&arg_parts);

    let edited_msg = ctx.translate_with_args(BasicStrings::PingPong.into(), &args);

    ctx.update_message(edited_msg, sent_msg.channel_id, sent_msg.id)
        .await?;

    Ok(())
}
