use std::sync::Arc;

use log::info;
use twilight::gateway::cluster::Event;
use twilight::model::gateway::payload::RequestGuildMembers;

use crate::core::Context;
use crate::utils::errors::Error;

pub async fn handle_event(
    shard_id: u64,
    event: &Event,
    ctx: Arc<Context>,
) -> Result<(), Error> {
    match &event {
        Event::GuildCreate(guild) => {
            ctx.stats.new_guild().await;
            let c = ctx.cluster.clone();
            let data = RequestGuildMembers::new_all(guild.id, None);
            info!("Requesting members for guild {}", guild.id);
            let res = tokio::spawn(async move { c.command(shard_id, &data).await }).await;

            if let Ok(handle) = res {
                match handle {
                    Ok(_) => return Ok(()),
                    Err(e) => return Err(Error::TwilightCluster(e)),
                }
            }
        }
        Event::MemberChunk(_chunk) => {}
        _ => (),
    }
    Ok(())
}
