use std::sync::atomic::Ordering;
use std::time::Duration;

use chrono::Utc;
use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder};

use crate::core::CommandContext;
use crate::error::{CommandResult, OtherFailure};
use crate::translation::{FluArgs, GearBotString};
use crate::utils::{self, Emoji};

const ABOUT_EMBED_COLOR: u32 = 0x00_cea2;

pub async fn about(ctx: CommandContext) -> CommandResult {
    let stats = &ctx.bot_context.stats;

    let shard_latency = ctx
        .bot_context
        .cluster
        .shard(ctx.shard)
        .unwrap()
        .info()
        .map_err(|e| OtherFailure::ShardOrCluster(e.to_string()))?
        .latency()
        .average()
        .unwrap_or_else(|| Duration::from_secs(0))
        .as_millis();

    let avg_latency = ctx
        .bot_context
        .cluster
        .info()
        .values()
        .map(|info| {
            info.latency()
                .average()
                .unwrap_or_else(|| Duration::default())
                .as_millis()
        })
        .sum::<u128>()
        / ctx.bot_context.scheme_info.shards_per_cluster as u128;

    let args = FluArgs::with_capacity(14)
        .add("gearDiamond", Emoji::GearDiamond.for_chat())
        .add("gearGold", Emoji::GearGold.for_chat())
        .add("gearIron", Emoji::GearIron.for_chat())
        .add("cluster_id", ctx.bot_context.scheme_info.cluster_id)
        .add("uptime", utils::age(ctx.bot_context.start_time, Utc::now(), 4))
        .add("start_time", ctx.bot_context.start_time.to_rfc2822())
        .add("version", stats.version)
        .add("shards", ctx.bot_context.scheme_info.total_shards)
        .add("average_latency", avg_latency)
        .add("guilds", stats.guild_counts.loaded.get())
        .add("total_users", stats.user_counts.total.get())
        .add("unique_users", stats.user_counts.unique.get())
        .add("shard", ctx.shard)
        .add("latency", shard_latency)
        .add("user_messages", stats.message_counts.user_messages.get())
        .add("messages_send", stats.message_counts.own_messages.get())
        .add("commands_executed", stats.total_command_counts.load(Ordering::Relaxed))
        .generate();

    let description = ctx.translate_with_args(GearBotString::AboutDescription, &args);

    let embed = EmbedBuilder::new()
        .description(description)?
        .color(ABOUT_EMBED_COLOR)?
        .timestamp(Utc::now().to_rfc3339())
        .field(
            EmbedFieldBuilder::new("Support Server", "[Click Here](https://discord.gg/PfwZmgU)")?
                .inline()
                .build(),
        )
        .field(
            EmbedFieldBuilder::new("Website", "[Click Here](https://gearbot.rocks)")?
                .inline()
                .build(),
        )
        .field(
            EmbedFieldBuilder::new("GitHub", "[Click Here](https://github.com/gearbot/GearBot)")?
                .inline()
                .build(),
        )
        .build()?;

    ctx.reply_embed(embed).await?;

    Ok(())
}
