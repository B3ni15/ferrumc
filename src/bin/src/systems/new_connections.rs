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
use tracing::debug;

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
        // already inserted a clone above; avoid moving the username again
        // Build AddPlayer packet for this new player including properties
    let short_uuid = new_connection.player_identity.short_uuid;
    // initial ping is 0 until we have an RTT measurement
    let add_player = PlayerWithActions::add_player_with_properties(short_uuid, new_connection.player_identity.username.clone(), 0);
    debug!("Broadcasting AddPlayer for player {:?}", new_connection.player_identity.username);
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
        // Existing-player send is handled in player_loaded; add-player broadcast done above
        if let Err(err) = return_sender.send(entity.id()) {
            error!(
                "Failed to send entity ID back to the networking thread: {:?}",
                err
            );
        }
    }
}
