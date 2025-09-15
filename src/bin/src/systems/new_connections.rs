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
                last_sent_instant: None,
                last_rtt_ms: 0,
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
    let add_entry = ferrumc_net::packets::outgoing::player_info_update::AddEntry {
        uuid: new_connection.player_identity.uuid.as_u128(),
        name: new_connection.player_identity.username.clone(),
        properties: {
            let mut lp = LengthPrefixedVec::default();
            match ferrumc_net::skins_cache::get_skin(short_uuid) {
                Some(sp) => lp.push(ferrumc_net::packets::outgoing::player_info_update::PlayerProperty { name: "textures".to_string(), value: sp.value, is_signed: sp.signature.is_some(), signature: sp.signature }),
                None => {}
            }
            lp
        },
        gamemode: ferrumc_net_codec::net_types::var_int::VarInt::new(0),
        ping: ferrumc_net_codec::net_types::var_int::VarInt::new(0),
        display_name: None,
    };
    debug!("Broadcasting AddPlayer for player {:?}", new_connection.player_identity.username);

    // Broadcast to other connected players
    for (other_entity, other_conn) in query.iter() {
        if other_entity == entity.id() {
            continue;
        }
        let mut buf = Vec::new();
        if let Err(e) = ferrumc_net::packets::outgoing::player_info_update::encode_full_packet(&mut buf, &ferrumc_net::packets::outgoing::player_info_update::PlayerInfoPacketKind::Add(vec![add_entry.clone()])) {
            tracing::warn!("Failed to encode add-player for {:?}: {:?}", other_entity, e);
            continue;
        }
        if let Err(e) = other_conn.send_raw(&buf) {
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
