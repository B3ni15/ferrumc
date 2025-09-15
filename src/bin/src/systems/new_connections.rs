use bevy_ecs::prelude::{Commands, Res, Resource};
use crossbeam_channel::Receiver;
use ferrumc_core::chunks::chunk_receiver::ChunkReceiver;
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_core::transform::grounded::OnGround;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::rotation::Rotation;
use ferrumc_inventories::hotbar::Hotbar;
use ferrumc_inventories::inventory::Inventory;
use ferrumc_net::connection::NewConnection;
use ferrumc_net::packets::outgoing::player_info_update::PlayerInfoUpdatePacket;
use ferrumc_net::packets::outgoing::player_info_update::PlayerWithActions;
use ferrumc_net::connection::StreamWriter;
use bevy_ecs::prelude::Query;
use ferrumc_state::GlobalStateResource;
use std::time::Instant;
use tracing::{error, trace};

#[derive(Resource)]
pub struct NewConnectionRecv(pub Receiver<NewConnection>);

pub fn accept_new_connections(
    mut cmd: Commands,
    new_connections: Res<NewConnectionRecv>,
    state: Res<GlobalStateResource>,
    query: Query<(bevy_ecs::prelude::Entity, &StreamWriter)>,
) {
    if new_connections.0.is_empty() {
        return;
    }
    while let Ok(new_connection) = new_connections.0.try_recv() {
        let return_sender = new_connection.entity_return;
        let entity = cmd.spawn((
            new_connection.stream,
            Position::default(),
            ChunkReceiver::default(),
            Rotation::default(),
            OnGround::default(),
            new_connection.player_identity.clone(),
            KeepAliveTracker {
                last_sent_keep_alive: 0,
                last_received_keep_alive: Instant::now(),
                has_received_keep_alive: true,
            },
            Inventory::new(46),
            Hotbar::default(),
        ));

        state.0.players.player_list.insert(
            entity.id(),
            (
                new_connection.player_identity.uuid.as_u128(),
                new_connection.player_identity.username.clone(),
            ),
        );

        trace!("Spawned entity for new connection: {:?}", entity.id());
        // Add the new entity to the global state
        state.0.players.player_list.insert(
            entity.id(),
            (
                new_connection.player_identity.uuid.as_u128(),
                new_connection.player_identity.username,
            ),
        );
        // Build AddPlayer packet for this new player including properties
        let short = new_connection.player_identity.uuid.as_u128() as i32;
        let add_player = PlayerWithActions::add_player_with_properties(short, new_connection.player_identity.username.clone());
        let add_packet = PlayerInfoUpdatePacket::with_players(vec![add_player]);

        // Broadcast to other connected players
        for (other_entity, other_conn) in query.iter() {
            if other_entity == entity.id() {
                continue;
            }
            if let Err(e) = other_conn.send_packet_ref(&add_packet) {
                tracing::warn!("Failed to send add-player to {:?}: {:?}", other_entity, e);
            }
        }
        // Send existing players to the newly connected client
        if let Some(stream_writer) = new_connection.stream.sender.clone().into_inner().ok() {
            // Note: we don't have direct access to Query here; instead use the PlayerInfoUpdatePacket::new_player_join_packet
            // to inform others and rely on other systems to send existing players to the new client when they finish loading.
        }

        // Broadcast to other players that a new player has joined
        // We will iterate over all connections in ECS and send the add-player packet
        // This requires access to the StreamWriter components; instead schedule a broadcast via the global state disconnection_queue is used elsewhere.
        if let Err(err) = return_sender.send(entity.id()) {
            error!(
                "Failed to send entity ID back to the networking thread: {:?}",
                err
            );
        }
    }
}
