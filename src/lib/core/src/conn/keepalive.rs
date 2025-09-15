use bevy_ecs::prelude::Component;
use std::time::Instant;

#[derive(Component)]
pub struct KeepAliveTracker {
    pub last_sent_keep_alive: i64,
    pub last_received_keep_alive: Instant,
    pub has_received_keep_alive: bool,
    // When we last sent a keep-alive packet (wall-clock instant). Used to compute RTT.
    pub last_sent_instant: Option<Instant>,
    // Most recently measured round-trip time in milliseconds.
    pub last_rtt_ms: i32,
}
