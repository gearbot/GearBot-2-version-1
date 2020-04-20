use log::{info, warn};
use twilight::gateway::cluster::Event;

use crate::gearbot_info;
use crate::core::logging;
use crate::utils::errors::Error;

pub async fn handle_event(shard_id: u64, event: &Event) -> Result<(), Error> {
    match &event {
        Event::ShardConnecting(_) => info!("Shard {} is connecting", shard_id),
        Event::ShardConnected(_) => gearbot_info!("Shard {} has connected", shard_id),
        Event::ShardDisconnected(_) => gearbot_info!("Shard {} has disconnected", shard_id),
        Event::ShardReconnecting(_) => {
            gearbot_info!("Shard {} is attempting to reconnect", shard_id)
        }
        Event::ShardResuming(_) => gearbot_info!("Shard {} is resuming", shard_id),
        Event::Ready(ready) => {
            logging::set_user(ready.user.clone());
            gearbot_info!("Connected to the gateway on shard {}!", shard_id)
        }
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
