use std::sync::Arc;
use std::time::{Duration, Instant};

use twilight::model::channel::Message;

use crate::commands::meta::nodes::CommandResult;
use crate::core::Context;
use crate::parser::Parser;
use crate::translation::BasicStrings;

pub async fn ping(ctx: Arc<Context>, msg: Message, _: Parser) -> CommandResult {
    let start = Instant::now();
    let sent_msg = ctx
        .http
        .create_message(msg.channel_id)
        .content(":ping_pong:")
        .await?;

    let finished = Instant::now();

    let rest_time = (finished - start).as_millis();

    let cluster_info = ctx.cluster.info().await;

    // This is 0 until we get a heartbeat
    let ws_time_avg = cluster_info
        .into_iter()
        .filter_map(|(_, info)| info.latency().average())
        .sum::<Duration>()
        .as_millis();

    let config_guard = ctx.get_config(msg.guild_id.unwrap()).await?;
    let config = config_guard.value();

    let arg_parts = [("rest", &rest_time), ("latency", &ws_time_avg)];
    let args = ctx.translations.generate_args(&arg_parts);

    let edited_msg =
        ctx.translations
            .get_text_with_args(&config.language, BasicStrings::PingPong.into(), &args);

    ctx.http
        .update_message(sent_msg.channel_id, sent_msg.id)
        .content(edited_msg.to_string())
        .await?;

    Ok(())
}
