use std::sync::Arc;

use log::debug;
use twilight_gateway::Event;

use crate::core::reactors::reactor_controller;
use crate::core::BotContext;
use crate::utils::Error;
use crate::{gearbot_info, gearbot_warn};

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<BotContext>) -> Result<(), Error> {
    match &event {
        Event::ShardReconnecting(_) => gearbot_info!("Shard {} is attempting to reconnect", shard_id),
        Event::ShardResuming(_) => gearbot_info!("Shard {} is resuming", shard_id),
        Event::Ready(_) => {
            gearbot_info!("Shard {} ready to go!", shard_id);
        }
        Event::GatewayInvalidateSession(recon) => {
            if *recon {
                gearbot_warn!(
                    "The gateway has invalidated our session for shard {}, but it is reconnectable!",
                    shard_id
                );
            } else {
                return Err(Error::InvalidSession(shard_id));
            }
        }
        Event::GatewayReconnect => gearbot_info!("Gateway requested shard {} to reconnect!", shard_id),
        Event::GatewayHello(u) => {
            debug!("Registered with gateway {} on shard {}", u, shard_id);
        }
        Event::Resumed => {
            gearbot_info!("Shard {} successfully resumed", shard_id);
        }
        Event::ReactionAdd(reaction) => {
            reactor_controller::process_reaction(&ctx, reaction).await?;
        }

        _ => (),
    }
    Ok(())
}
