use bevy_ecs::prelude::{Entity, Query, Res, ResMut};
use ferrumc_core::conn::player_count_update_cooldown::PlayerCountUpdateCooldown;
use ferrumc_core::identity::player_identity::PlayerIdentity;
use ferrumc_state::GlobalStateResource;
use ferrumc_core::conn::keepalive::KeepAliveTracker;
use ferrumc_net::connection::StreamWriter;
use ferrumc_net::packets::outgoing::player_info_update::{PlayerInfoUpdatePacket, PlayerWithActions};
use tracing::warn;

pub fn player_count_updater(
    state: Res<GlobalStateResource>,
    player_query: Query<(Entity, &PlayerIdentity, &KeepAliveTracker)>,
    all_players_stream_writer_query: Query<&StreamWriter>,
    mut cooldown_tracker: ResMut<PlayerCountUpdateCooldown>,
) {
    // Frequency is controlled by the schedule period.
    for (entity, player_identity, keep_alive_tracker) in player_query.iter() {
        let uuid = player_identity.short_uuid;

        let player_info_packet = PlayerInfoUpdatePacket::with_players(vec![
            PlayerWithActions::update_latency(uuid, keep_alive_tracker.ping),
        ]);

        for stream_writer in all_players_stream_writer_query.iter() {
            if let Err(err) = stream_writer.send_packet_ref(&player_info_packet) {
                warn!("Failed to send player info update packet to {}: {:?}", entity, err);
            }
        }
    }
    cooldown_tracker.last_update = std::time::Instant::now();
}
