use std::sync::atomic::Ordering;
use std::time::Duration;

use chrono::Utc;
use twilight_embed_builder::{EmbedBuilder, EmbedFieldBuilder};

use crate::commands::meta::nodes::CommandResult;
use crate::core::CommandContext;
use crate::translation::{FluArgs, GearBotString};
use crate::utils::{age, Emoji, OtherFailure};

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
        .insert("gearDiamond", Emoji::GearDiamond.for_chat())
        .insert("gearGold", Emoji::GearGold.for_chat())
        .insert("gearIron", Emoji::GearIron.for_chat())
        .insert("cluster_id", ctx.bot_context.scheme_info.cluster_id)
        .insert("uptime", age(ctx.bot_context.start_time, Utc::now(), 4))
        .insert("start_time", ctx.bot_context.start_time.to_rfc2822())
        .insert("version", stats.version)
        .insert("shards", ctx.bot_context.scheme_info.total_shards)
        .insert("average_latency", avg_latency)
        .insert("guilds", stats.guild_counts.loaded.get())
        .insert("total_users", stats.user_counts.total.get())
        .insert("unique_users", stats.user_counts.unique.get())
        .insert("shard", ctx.shard)
        .insert("latency", shard_latency)
        .insert("user_messages", stats.message_counts.user_messages.get())
        .insert("messages_send", stats.message_counts.own_messages.get())
        .insert("commands_executed", stats.total_command_counts.load(Ordering::Relaxed))
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
