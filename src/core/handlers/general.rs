use std::sync::Arc;

use log::debug;
use twilight::gateway::Event;
use twilight::model::gateway::presence::{Activity, ActivityType, Status};

use crate::core::BotContext;
use crate::utils::Error;
use crate::{gearbot_info, gearbot_warn};

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<BotContext>) -> Result<(), Error> {
    match &event {
        Event::ShardReconnecting(_) => gearbot_info!("Shard {} is attempting to reconnect", shard_id),
        Event::ShardResuming(_) => gearbot_info!("Shard {} is resuming", shard_id),
        Event::Ready(_) => {
            gearbot_info!("Shard {} ready to go!", shard_id);
            ctx.set_shard_activity(
                shard_id,
                Status::Online,
                ActivityType::Watching,
                String::from("the gears turn"),
            )
            .await?
        }
        Event::GatewayInvalidateSession(recon) => {
            if *recon {
                gearbot_warn!("The gateway has invalidated our session, but it is reconnectable!");
            } else {
                return Err(Error::InvalidSession);
            }
        }
        Event::GatewayReconnect => gearbot_info!("Gateway requested shard {} to reconnect!", shard_id),
        Event::GatewayHello(u) => {
            debug!("Registered with gateway {} on shard {}", u, shard_id);
            ctx.set_shard_activity(
                shard_id,
                Status::Idle,
                ActivityType::Listening,
                String::from("to the modem screeking as i connect to the gateway"),
            )
            .await?
        }
        Event::Resumed => {
            gearbot_info!("Shard {} successfully resumed", shard_id);
            ctx.set_shard_activity(
                shard_id,
                Status::Online,
                ActivityType::Watching,
                String::from("the gears turn"),
            )
            .await?
        }
        Event::MemberChunk(_chunk) => {
            // debug!("got a chunk with nonce {:?}", &chunk.nonce);
        }
        _ => (),
    }
    Ok(())
}
