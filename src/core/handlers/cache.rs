use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;
use twilight::model::gateway::payload::RequestGuildMembers;

use crate::core::Context;
use crate::Error;

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<Context<'_>>) -> Result<(), Error> {
    match &event {
        Event::GuildCreate(guild) => {
            ctx.stats.new_guild().await;
            let c = ctx.cluster.clone();
            let data = RequestGuildMembers::new_all(guild.id, None);
            info!("Requesting members for guild {}", guild.id);
            tokio::spawn(async move {
                c.command(shard_id, &data).await;
            });
            ()
        },
        Event::MemberChunk(chunk) => {
        }
        _ => (),
    }
    Ok(())
}