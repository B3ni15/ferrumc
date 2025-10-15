use bevy_ecs::prelude::{Component, Entity, Query};
use ferrumc_core::identity::player_identity::PlayerIdentity;
use ferrumc_macros::{packet, NetEncode};
use ferrumc_net_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use ferrumc_net_codec::net_types::var_int::VarInt;
use tracing::debug;

#[derive(NetEncode)]
#[packet(packet_id = "player_info_update", state = "play")]
pub struct PlayerInfoUpdatePacket {
    pub actions: u8,
    pub numbers_of_players: VarInt,
    pub players: Vec<PlayerWithActions>,
}

impl PlayerInfoUpdatePacket {
    pub fn with_players(players: Vec<PlayerWithActions>) -> Self {
        let players: Vec<PlayerWithActions> = players.into_iter().collect();
        Self {
            actions: players
                .iter()
                .map(|player| player.get_actions_mask())
                .fold(0, |acc, x| acc | x),
            numbers_of_players: VarInt::new(players.len() as i32),
            players,
        }
    }

    /// The packet to be sent to all already connected players when a new player joins the server
    pub fn new_player_join_packet(identity: PlayerIdentity, ping: i32) -> Self {
        let player = PlayerWithActions::add_player(identity.short_uuid, identity.username, ping);

        Self::with_players(vec![player])
    }

    /// The packet to be sent to a new player when they join the server,
    /// To let them know about all the players that are already connected
    pub fn existing_player_info_packet(
        new_player_id: Entity,
        query: Query<(Entity, &PlayerIdentity, &KeepAliveTracker)>,
    ) -> Self {
        let players: Vec<(i32, String, i32)> = query
            .iter()
            .filter(|&(entity, _, _)| entity != new_player_id)
            .map(|(_, player_identity, keep_alive_tracker)| {
                (
                    player_identity.short_uuid,
                    player_identity.username.clone(),
                    keep_alive_tracker.ping,
                )
            })
            .collect();

        let players = players
            .into_iter()
            .map(|(uuid, name, ping)| PlayerWithActions::add_player(uuid, name, ping))
            .collect::<Vec<_>>();

        debug!("Sending PlayerInfoUpdatePacket with {:?} players", players);

        Self::with_players(players)
    }
}

#[derive(NetEncode, Debug, Component)]
pub struct PlayerWithActions {
    pub uuid: i32,
    pub actions: Vec<PlayerAction>,
}

impl PlayerWithActions {
    pub fn get_actions_mask(&self) -> u8 {
        let mut mask = 0;
        for action in &self.actions {
            mask |= match action {
                PlayerAction::AddPlayer { .. } => 0x01,
                PlayerAction::UpdateGamemode { .. } => 0x02,
                PlayerAction::UpdateListed { .. } => 0x08,
                PlayerAction::UpdateLatency { .. } => 0x04,
            }
        }
        mask
    }

    pub fn add_player(uuid: i32, name: impl Into<String>, ping: i32) -> Self {
        Self {
            uuid,
            actions: vec![PlayerAction::AddPlayer {
                name: name.into(),
                properties: LengthPrefixedVec::default(),
                ping,
            }],
        }
    }

    pub fn update_gamemode(uuid: i32, gamemode: i32) -> Self {
        Self {
            uuid,
            actions: vec![PlayerAction::UpdateGamemode {
                gamemode: VarInt::new(gamemode),
            }],
        }
    }

    pub fn update_listed(uuid: i32, listed: bool) -> Self {
        Self {
            uuid,
            actions: vec![PlayerAction::UpdateListed { listed }],
        }
    }

    pub fn update_latency(uuid: i32, ping: i32) -> Self {
        Self {
            uuid,
            actions: vec![PlayerAction::UpdateLatency { ping: VarInt::new(ping) }],
        }
    }
}

#[derive(NetEncode, Debug)]
pub enum PlayerAction {
    AddPlayer {
        name: String,
        properties: LengthPrefixedVec<PlayerProperty>,
        ping: i32,
    },
    UpdateGamemode {
        gamemode: VarInt,
    },
    UpdateListed {
        listed: bool,
    },
    UpdateLatency {
        ping: VarInt,
    },
}

#[derive(NetEncode, Debug)]
pub struct PlayerProperty {
    pub name: String,
    pub value: String,
    pub is_signed: bool,
    pub signature: Option<String>,
}
