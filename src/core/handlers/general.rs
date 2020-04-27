use std::sync::Arc;

use log::{debug, info, warn};
use twilight::gateway::cluster::Event;
use twilight::model::gateway::payload::UpdateStatus;
use twilight::model::gateway::presence::{Activity, ActivityType, Status};

use crate::core::{Context, logging};
use crate::gearbot_info;
use crate::utils::Error;

pub async fn handle_event(shard_id: u64, event: &Event, ctx: Arc<Context>) -> Result<(), Error> {
    match &event {
        Event::ShardConnecting(_) => info!("Shard {} is connecting", shard_id),
        Event::ShardConnected(_) => gearbot_info!("Shard {} has connected", shard_id),
        Event::ShardDisconnected(_) => gearbot_info!("Shard {} has disconnected", shard_id),
        Event::ShardReconnecting(_) => {
            gearbot_info!("Shard {} is attempting to reconnect", shard_id)
        }
        Event::ShardResuming(_) => gearbot_info!("Shard {} is resuming", shard_id),
        Event::Ready(ready) => {
            gearbot_info!("Shard {} ready to go!", shard_id);
            ctx.cluster
                .command(
                    shard_id,
                    &UpdateStatus::new(
                        false,
                        gen_activity(String::from("the gears turn")),
                        None,
                        Status::Online,
                    ),
                )
                .await?;
        }
        Event::GatewayInvalidateSession(recon) => {
            if *recon {
                warn!("The gateway has invalidated our session, but it is reconnectable!");
            } else {
                return Err(Error::InvalidSession);
            }
        }
        Event::GatewayReconnect => info!("We reconnected to the gateway!"),
        Event::GatewayHello(u) => {
            debug!("Registered with gateway {} on shard {}", u, shard_id);
            ctx.cluster
                .command(
                    shard_id,
                    &UpdateStatus::new(
                        true,
                        gen_activity(String::from("things coming online")),
                        None,
                        Status::Idle,
                    ),
                )
                .await?;
        }
        _ => (),
    }
    Ok(())
}

fn gen_activity(name: String) -> Activity {
    Activity {
        application_id: None,
        assets: None,
        created_at: None,
        details: None,
        flags: None,
        id: None,
        instance: None,
        kind: ActivityType::Watching,
        name,
        emoji: None,
        party: None,
        secrets: None,
        state: None,
        timestamps: None,
        url: None,
    }
}
