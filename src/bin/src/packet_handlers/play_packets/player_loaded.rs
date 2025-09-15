use bevy_ecs::prelude::{Entity, Query, Res};
use ferrumc_core::transform::position::Position;
use ferrumc_net::connection::StreamWriter;
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_net::packets::outgoing::player_info_update::PlayerInfoUpdatePacket;
use ferrumc_net::packets::outgoing::synchronize_player_position::SynchronizePlayerPositionPacket;
use ferrumc_net::PlayerLoadedReceiver;
use ferrumc_state::GlobalStateResource;
use ferrumc_world::block_id::BlockId;
use tracing::warn;
use std::collections::HashMap;

pub fn handle(
    ev: Res<PlayerLoadedReceiver>,
    state: Res<GlobalStateResource>,
    query: Query<(Entity, &Position, &StreamWriter, &KeepAliveTracker)>,
) {
    for (_, player) in ev.0.try_iter() {
        let Ok((entity, player_pos, conn, keepalive)) = query.get(player) else {
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
        // Build AddEntry list and fill ping using a snapshot map of Entity -> last_rtt_ms
        let mut add_entries = vec![];

        // Build a quick lookup map from entities in the incoming query to their last_rtt_ms
        let mut ping_map: HashMap<bevy_ecs::prelude::Entity, i32> = HashMap::new();
        for (ent, _pos, _conn, tracker) in query.iter() {
            ping_map.insert(ent, tracker.last_rtt_ms);
        }

        for entry in state.0.players.player_list.iter() {
            let (entity_key, (uuid128, name)) = (entry.key().clone(), entry.value().clone());
            let ping_ms = ping_map.get(&entity_key).copied().unwrap_or(0);
            let mut props = LengthPrefixedVec::default();
            if let Some(sp) = ferrumc_net::skins_cache::get_skin(uuid128 as i32) {
                props.push(ferrumc_net::packets::outgoing::player_info_update::PlayerProperty { name: "textures".to_string(), value: sp.value, is_signed: sp.signature.is_some(), signature: sp.signature });
            }
            add_entries.push(ferrumc_net::packets::outgoing::player_info_update::AddEntry { uuid: uuid128, name, properties: props, gamemode: ferrumc_net_codec::net_types::var_int::VarInt::new(0), ping: ferrumc_net_codec::net_types::var_int::VarInt::new(ping_ms), display_name: None });
        }

        if !add_entries.is_empty() {
            tracing::debug!("Sending existing players list to {}: {} entries", player, add_entries.len());
            let mut buf = Vec::new();
            if let Err(e) = ferrumc_net::packets::outgoing::player_info_update::encode_full_packet(&mut buf, &ferrumc_net::packets::outgoing::player_info_update::PlayerInfoPacketKind::Add(add_entries)) {
                tracing::error!("Failed to encode existing players for {}: {:?}", player, e);
            } else if let Err(e) = conn.send_raw(&buf) {
                tracing::error!("Failed to send existing players to {}: {:?}", player, e);
            }
        }

        // Add broadcast already sent at spawn in `accept_new_connections`; do not re-broadcast here
    }
}
