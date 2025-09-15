use bevy_ecs::prelude::{Commands, Entity, Query, Res};
use ferrumc_core::identity::player_identity::PlayerIdentity;
use ferrumc_net::connection::StreamWriter;
use ferrumc_state::GlobalStateResource;
use ferrumc_text::TextComponent;
use tracing::{info, trace, warn};

pub fn connection_killer(
    query: Query<(Entity, &StreamWriter, &PlayerIdentity)>,
    mut cmd: Commands,
    state: Res<GlobalStateResource>,
) {
    while let Some((disconnecting_entity, reason)) = state.0.players.disconnection_queue.pop() {
        for (entity, conn, player_identity) in query.iter() {
            if disconnecting_entity == entity {
                info!(
                    "Player {} ({}) disconnected: {}",
                    player_identity.username,
                    player_identity.uuid,
                    reason.as_deref().unwrap_or("No reason")
                );
                if conn.running.load(std::sync::atomic::Ordering::Relaxed) {
                    trace!(
                        "Sending disconnect packet to player {}",
                        player_identity.username
                    );
                    if let Err(e) = conn.send_packet_ref(
                        &ferrumc_net::packets::outgoing::disconnect::DisconnectPacket {
                            reason: TextComponent::from(
                                reason.as_deref().unwrap_or("Disconnected"),
                            ),
                        },
                    ) {
                        warn!(
                            "Failed to send disconnect packet to player {}: {:?}",
                            player_identity.username, e
                        );
                    }
                } else {
                    trace!(
                        "Connection for player {} is not running, skipping disconnect packet",
                        player_identity.username
                    );
                }
            } else {
                // Broadcast the disconnection to other players
                // Build RemovePlayer packet and send to others
                if let Some((uuid128, _name)) = state.0.players.player_list.get(&disconnecting_entity).map(|v| *v.value()) {
                    let short = uuid128 as i32;
                    let remove_pkt = ferrumc_net::packets::outgoing::player_info_update::PlayerInfoUpdatePacket::with_players(vec![
                        ferrumc_net::packets::outgoing::player_info_update::PlayerWithActions::remove_player(short)
                    ]);
                    for (other_entity, other_conn, _other_identity) in query.iter() {
                        if other_entity == disconnecting_entity {
                            continue;
                        }
                        if let Err(e) = other_conn.send_packet_ref(&remove_pkt) {
                            warn!("Failed to send remove-player packet to {:?}: {:?}", other_entity, e);
                        }
                    }
                }
            }
            cmd.entity(entity).despawn();
        }
    }
}
