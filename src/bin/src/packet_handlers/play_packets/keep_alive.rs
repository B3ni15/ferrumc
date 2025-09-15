use bevy_ecs::prelude::Res;
use bevy_ecs::system::Query;
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_net::IncomingKeepAlivePacketReceiver;
use ferrumc_state::GlobalStateResource;
use std::time::Instant;
use tracing::{error, warn};

pub fn handle(
    events: Res<IncomingKeepAlivePacketReceiver>,
    mut query: Query<&mut KeepAliveTracker>,
    state: Res<GlobalStateResource>,
) {
    for (event, eid) in events.0.try_iter() {
        let Ok(mut keep_alive_tracker) = query.get_mut(eid) else {
            error!("Could not get keep alive tracker for entity {:?}", eid);
            continue;
        };
        if event.timestamp != keep_alive_tracker.last_sent_keep_alive {
            warn!(
                "Invalid keep alive packet received from {:?} with id {:?} (expected {:?})",
                eid, event.timestamp, keep_alive_tracker.last_sent_keep_alive
            );
            state
                .0
                .players
                .disconnect(eid, Some("Invalid keep alive packet received".to_string()));
        } else {
            // compute RTT if we have an instant for when the packet was sent
            let now = Instant::now();
            if let Some(sent_instant) = keep_alive_tracker.last_sent_instant {
                let rtt = now.duration_since(sent_instant);
                keep_alive_tracker.last_rtt_ms = rtt.as_millis().clamp(0, i128::from(i32::MAX) as u128) as i32;
            }

            keep_alive_tracker.last_received_keep_alive = Instant::now();
            keep_alive_tracker.has_received_keep_alive = true;
        }
    }
}
