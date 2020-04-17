use std::sync::Arc;

use log::{info, warn};
use twilight::gateway::cluster::Event;

use crate::{Error, gearbot_info};
use crate::core::Context;

pub async fn handle_event(shard_id: &u64, event: &Event, ctx: Arc<Context<'_>>) -> Result<(), Error> {
    match &event {
        Event::ShardConnected(_) => gearbot_info!("Shard {} has connected", shard_id),
        Event::ShardDisconnected(_) => gearbot_info!("Shard {} has disconnected", shard_id),
        Event::ShardReconnecting(_) => gearbot_info!("Shard {} is attempting to reconnect", shard_id),
        Event::ShardResuming(_) => gearbot_info!("Shard {} is resuming itself", shard_id),
        Event::Ready(ready) => gearbot_info!("Connected to the gateway as {}", ready.user.name),
        Event::GatewayInvalidateSession(recon) => {
            if *recon {
                warn!("The gateway has invalidated our session, but it is reconnectable!");
            } else {
                return Err(Error::InvalidSession);
            }
        }
        Event::GatewayReconnect => info!("We reconnected to the gateway!"),
        Event::GatewayHello(u) => info!("Registered with gateway {}", u),

        _ => (),
    }
    Ok(())
}