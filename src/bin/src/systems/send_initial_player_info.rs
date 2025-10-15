use bevy_ecs::prelude::{Commands, Entity, Query, Res, With};
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_core::conn::new_player_tag::NewPlayerTag;
use ferrumc_core::identity::player_identity::PlayerIdentity;
use ferrumc_net::connection::StreamWriter;
use ferrumc_net::packets::outgoing::player_info_update::{PlayerInfoUpdatePacket, PlayerWithActions};
use tracing::warn;

pub fn send_initial_player_info(
    mut cmd: Commands,
    new_players_query: Query<
        (Entity, &PlayerIdentity, &StreamWriter, &KeepAliveTracker),
        With<NewPlayerTag>,
    >,
    all_players_query: Query<(Entity, &PlayerIdentity, &StreamWriter, &KeepAliveTracker)>,
) {
    for (new_player_entity, new_player_identity, new_player_stream_writer, new_player_keep_alive_tracker) in
        new_players_query.iter()
    {
        // Send new player info to all existing players
        let new_player_join_packet = PlayerInfoUpdatePacket::new_player_join_packet(
            new_player_identity.clone(),
            new_player_keep_alive_tracker.ping,
        );
        for (existing_player_entity, _, existing_player_stream_writer, _) in all_players_query.iter() {
            if existing_player_entity != new_player_entity {
                if let Err(err) = existing_player_stream_writer.send_packet_ref(&new_player_join_packet) {
                    warn!(
                        "Failed to send new player join packet to existing player {}: {:?}",
                        existing_player_entity,
                        err
                    );
                }
            }
        }

        // Send existing players info to the new player
        let existing_players_actions: Vec<PlayerWithActions> = all_players_query
            .iter()
            .filter(|&(e, _, _)| e != new_player_entity)
            .map(|(_, player_identity, keep_alive_tracker)| {
                PlayerWithActions::add_player(
                    player_identity.short_uuid,
                    player_identity.username.clone(),
                    keep_alive_tracker.ping,
                )
            })
            .collect();

        let existing_players_info_packet = PlayerInfoUpdatePacket::with_players(existing_players_actions);
        if let Err(err) = new_player_stream_writer.send_packet_ref(&existing_players_info_packet) {
            warn!(
                "Failed to send existing players info packet to new player {}: {:?}",
                new_player_entity,
                err
            );
        }

        // Remove the NewPlayerTag from the new player
        cmd.entity(new_player_entity).remove::<NewPlayerTag>();
    }
}
