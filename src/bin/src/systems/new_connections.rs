use bevy_ecs::prelude::{Commands, Res, Resource, Entity, Query};
use crossbeam_channel::Receiver;
use ferrumc_core::chunks::chunk_receiver::ChunkReceiver;
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_core::transform::grounded::OnGround;
use ferrumc_core::transform::position::Position;
use ferrumc_core::transform::rotation::Rotation;
use ferrumc_inventories::hotbar::Hotbar;
use ferrumc_inventories::inventory::Inventory;
use ferrumc_net::connection::{NewConnection, StreamWriter};
use ferrumc_net::packets::outgoing::player_info_update::{PlayerInfoUpdatePacket, PlayerWithActions};
use ferrumc_state::GlobalStateResource;
use std::time::Instant;
use tracing::{error, trace, warn};
use ferrumc_core::identity::player_identity::PlayerIdentity;

#[derive(Resource)]
pub struct NewConnectionRecv(pub Receiver<NewConnection>);

pub fn accept_new_connections(
    mut cmd: Commands,
    new_connections: Res<NewConnectionRecv>,
    state: Res<GlobalStateResource>,
    all_players_query: Query<(Entity, &PlayerIdentity, &StreamWriter, &KeepAliveTracker)>,
) {
    if new_connections.0.is_empty() {
        return;
    }
    while let Ok(new_connection) = new_connections.0.try_recv() {
        let return_sender = new_connection.entity_return;
        let player_identity = new_connection.player_identity.clone();

        let entity = cmd.spawn((
            new_connection.stream,
            Position::default(),
            ChunkReceiver::default(),
            Rotation::default(),
            OnGround::default(),
            player_identity.clone(),
            KeepAliveTracker {
                last_sent_keep_alive: 0,
                last_received_keep_alive: Instant::now(),
                has_received_keep_alive: true,
                ping: 0,
                last_sent_instant: Instant::now(),
            },
            Inventory::new(46),
            Hotbar::default(),
        )).id();

        state.0.players.player_list.insert(
            entity,
            (
                player_identity.uuid.as_u128(),
                player_identity.username.clone(),
            ),
        );

        trace!("Spawned entity for new connection: {:?}", entity);

        // Get the StreamWriter for the newly spawned entity
        let (_, _, new_player_stream_writer, _) = all_players_query.get(entity).unwrap();

        // Send new player info to all existing players
        let new_player_join_packet = PlayerInfoUpdatePacket::new_player_join_packet(player_identity.clone(), 0);
        for (existing_player_entity, _, stream_writer, _) in all_players_query.iter() {
            if existing_player_entity != entity {
                if let Err(err) = stream_writer.send_packet_ref(&new_player_join_packet) {
                    warn!("Failed to send new player join packet to existing player {}: {:?}", existing_player_entity, err);
                }
            }
        }

        // Send existing players info to the new player
        let existing_players_actions: Vec<PlayerWithActions> = all_players_query
            .iter()
            .filter(|&(e, _, _, _)| e != entity)
            .map(|(_, player_identity, _, keep_alive_tracker)| {
                PlayerWithActions::add_player(
                    player_identity.short_uuid,
                    player_identity.username.clone(),
                    keep_alive_tracker.ping,
                )
            })
            .collect();

        let existing_players_info_packet = PlayerInfoUpdatePacket::with_players(existing_players_actions);
        if let Err(err) = new_player_stream_writer.send_packet_ref(&existing_players_info_packet) {
            warn!("Failed to send existing players info packet to new player {}: {:?}", entity, err);
        }

        if let Err(err) = return_sender.send(entity) {
            error!(
                "Failed to send entity ID back to the networking thread: {:?}",
                err
            );
        }
    }
}
