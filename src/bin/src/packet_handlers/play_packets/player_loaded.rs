use bevy_ecs::prelude::{Entity, Query, Res};
use ferrumc_core::transform::position::Position;
use ferrumc_net::connection::StreamWriter;
use ferrumc_net::packets::outgoing::player_info_update::PlayerInfoUpdatePacket;
use ferrumc_net::packets::outgoing::synchronize_player_position::SynchronizePlayerPositionPacket;
use ferrumc_net::PlayerLoadedReceiver;
use ferrumc_state::GlobalStateResource;
use ferrumc_world::block_id::BlockId;
use tracing::warn;

pub fn handle(
    ev: Res<PlayerLoadedReceiver>,
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &Position, &StreamWriter)>,
) {
    for (_, player) in ev.0.try_iter() {
        let Ok((entity, player_pos, conn)) = query.get(player) else {
            warn!("Player position not found in query.");
            continue;
        };
        if !state.0.players.is_connected(entity) {
            warn!(
                "Player {} is not connected, skipping position synchronization.",
                player
            );
            continue;
        }
        let head_block = state.0.world.get_block_and_fetch(
            player_pos.x as i32,
            player_pos.y as i32,
            player_pos.z as i32,
            "overworld",
        );
        if let Ok(head_block) = head_block {
            if head_block == BlockId(0) {
                tracing::info!(
                    "Player {} loaded at position: ({}, {}, {})",
                    player,
                    player_pos.x,
                    player_pos.y,
                    player_pos.z
                );
            } else {
                tracing::info!(
                    "Player {} loaded at position: ({}, {}, {}) with head block: {:?}",
                    player,
                    player_pos.x,
                    player_pos.y,
                    player_pos.z,
                    head_block
                );
                // Teleport the player to the world center if their head block is not air
                let packet = SynchronizePlayerPositionPacket::default();
                if let Err(e) = conn.send_packet_ref(&packet) {
                    tracing::error!(
                        "Failed to send synchronize player position packet for player {}: {:?}",
                        player,
                        e
                    );
                } else {
                    tracing::info!(
                        "Sent synchronize player position packet for player {}",
                        player
                    );
                }
            }
        } else {
            warn!(
                "Failed to fetch head block for player {} at position: ({}, {}, {})",
                player, player_pos.x, player_pos.y, player_pos.z
            );
        }

        // Send existing players to this newly loaded player (including themselves)
        // Build a packet that contains all currently connected players
        let mut players = vec![];
        for entry in state.0.players.player_list.iter() {
            let (_entity, (uuid128, name)) = (entry.key().clone(), entry.value().clone());
            // short uuid is i32 (lower bits)
            let short = uuid128 as i32;
            players.push(ferrumc_net::packets::outgoing::player_info_update::PlayerWithActions::add_player_with_properties(short, name.clone()));
        }

        if !players.is_empty() {
            tracing::debug!("Sending existing players list to {}: {} entries", player, players.len());
            let packet = PlayerInfoUpdatePacket::with_players(players);
            if let Err(e) = conn.send_packet_ref(&packet) {
                tracing::error!("Failed to send existing players to {}: {:?}", player, e);
            }
        }

        // Add broadcast already sent at spawn in `accept_new_connections`; do not re-broadcast here
    }
}
