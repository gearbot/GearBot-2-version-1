use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;

pub async fn ping(ctx: Arc<Context>, msg: Message, _: Parser) -> CommandResult {
    let start = Utc::now().time();
    let sent_msg = ctx
        .http
        .create_message(msg.channel_id)
        .content(":ping_pong:")
        .await
        .unwrap();

    let finished = Utc::now().time();

    let rest_time = (finished - start).num_milliseconds();

    let cluster_info = ctx.cluster.info().await;

    // This is 0 until we get a heartbeat
    let ws_time_avg = cluster_info
        .into_iter()
        .filter_map(|(_, info)| info.latency().average())
        .sum::<Duration>()
        .as_millis();

    let edited_msg = format!(
        ":hourglass: REST API ping is {} ms | Websocket ping is {} ms :hourglass:",
        rest_time, ws_time_avg
    );

    ctx.http
        .update_message(sent_msg.channel_id, sent_msg.id)
        .content(edited_msg)
        .await?;

    Ok(())
}
