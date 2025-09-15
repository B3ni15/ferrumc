use ferrumc_net_codec::net_types::length_prefixed_vec::LengthPrefixedVec;
use bevy_ecs::prelude::Component;
use ferrumc_core::identity::player_identity::PlayerIdentity;
use ferrumc_macros::packet;
use ferrumc_net_codec::net_types::var_int::VarInt;
use ferrumc_text::TextComponent;
use ferrumc_net_codec::encode::NetEncode;
use ferrumc_net_codec::encode::NetEncodeOpts;
use ferrumc_net_codec::encode::errors::NetEncodeError;
use std::io::Write;
use tracing::debug;
use crate::skins_cache;

#[packet(packet_id = "player_info_update", state = "play")]
pub struct PlayerInfoUpdatePacket {
    // This struct is a placeholder to register the packet id with the macro.
    // Actual encoding is implemented in `impl NetEncode for PlayerInfoUpdatePacket` below.
    pub action: VarInt,
    pub numbers_of_players: VarInt,
}

#[derive(Debug, Clone, Component)]
pub struct AddEntry {
    pub uuid: u128,
    pub name: String,
    pub properties: LengthPrefixedVec<PlayerProperty>,
    pub gamemode: VarInt,
    pub ping: VarInt,
    pub display_name: Option<TextComponent>,
}

#[derive(Debug, Clone)]
pub struct UpdateLatencyEntry {
    pub uuid: u128,
    pub ping: VarInt,
}

#[derive(Debug, Clone)]
pub struct RemoveEntry {
    pub uuid: u128,
}

#[derive(Debug, Clone)]
pub struct PlayerProperty {
    pub name: String,
    pub value: String,
    pub is_signed: bool,
    pub signature: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PlayerInfoPacketKind {
    Add(Vec<AddEntry>),
    UpdateLatency(Vec<UpdateLatencyEntry>),
    Remove(Vec<RemoveEntry>),
}

// Wrapper that implements NetEncode for a full Player Info packet (writes packet id + body).
#[derive(Debug, Clone)]
pub struct PlayerInfoFull(pub PlayerInfoPacketKind);

impl NetEncode for PlayerInfoFull {
    fn encode<W: std::io::Write>(&self, writer: &mut W, _opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        // Packet id for `player_info_update` in our assets mapping is 63
        VarInt::new(63).encode(writer, &NetEncodeOpts::None)?;
        match &self.0 {
            PlayerInfoPacketKind::Add(entries) => {
                // action 0, number of players
                VarInt::new(0).encode(writer, &NetEncodeOpts::None)?;
                VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
                for e in entries {
                    e.uuid.encode(writer, &NetEncodeOpts::None)?;
                    e.name.encode(writer, &NetEncodeOpts::None)?;
                    e.properties.encode(writer, &NetEncodeOpts::None)?;
                    e.gamemode.encode(writer, &NetEncodeOpts::None)?;
                    e.ping.encode(writer, &NetEncodeOpts::None)?;
                    match &e.display_name {
                        Some(text) => {
                            true.encode(writer, &NetEncodeOpts::None)?;
                            text.encode(writer, &NetEncodeOpts::None)?;
                        }
                        None => {
                            false.encode(writer, &NetEncodeOpts::None)?;
                        }
                    }
                }
                Ok(())
            }
            PlayerInfoPacketKind::UpdateLatency(entries) => {
                VarInt::new(2).encode(writer, &NetEncodeOpts::None)?;
                VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
                for e in entries {
                    e.uuid.encode(writer, &NetEncodeOpts::None)?;
                    e.ping.encode(writer, &NetEncodeOpts::None)?;
                }
                Ok(())
            }
            PlayerInfoPacketKind::Remove(entries) => {
                VarInt::new(4).encode(writer, &NetEncodeOpts::None)?;
                VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
                for e in entries {
                    e.uuid.encode(writer, &NetEncodeOpts::None)?;
                }
                Ok(())
            }
        }
    }

    async fn encode_async<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        _writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        unimplemented!();
    }
}

impl PlayerInfoUpdatePacket {
    pub fn add_players(entries: Vec<AddEntry>) -> (Self, PlayerInfoPacketKind) {
        let pkt = Self { action: VarInt::new(0), numbers_of_players: VarInt::new(entries.len() as i32) };
        (pkt, PlayerInfoPacketKind::Add(entries))
    }

    pub fn update_latency(entries: Vec<UpdateLatencyEntry>) -> (Self, PlayerInfoPacketKind) {
        let pkt = Self { action: VarInt::new(2), numbers_of_players: VarInt::new(entries.len() as i32) };
        (pkt, PlayerInfoPacketKind::UpdateLatency(entries))
    }

    pub fn remove_players(entries: Vec<RemoveEntry>) -> (Self, PlayerInfoPacketKind) {
        let pkt = Self { action: VarInt::new(4), numbers_of_players: VarInt::new(entries.len() as i32) };
        (pkt, PlayerInfoPacketKind::Remove(entries))
    }
}

impl NetEncode for PlayerInfoUpdatePacket {
    fn encode<W: Write>(&self, writer: &mut W, _opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        // This implementation expects the caller to first write the action and number of players,
        // then for each player write the appropriate fields depending on action. However, because
        // the macro-generated code expects to call `encode` on the packet only, we will not implement
        // a full per-entry encode here; instead callers should use the helper `encode_full_packet` below.
        // To keep compatibility with the macro, encode only the action and number_of_players.
        self.action.encode(writer, &NetEncodeOpts::None)?;
        self.numbers_of_players.encode(writer, &NetEncodeOpts::None)?;
        Ok(())
    }

    async fn encode_async<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        _writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        // Async encoding not needed for this packet
        unimplemented!();
    }
}

impl NetEncode for PlayerProperty {
    fn encode<W: Write>(&self, writer: &mut W, _opts: &NetEncodeOpts) -> Result<(), NetEncodeError> {
        // name, value, has_signature(bool), signature if present
        self.name.encode(writer, &NetEncodeOpts::None)?;
        self.value.encode(writer, &NetEncodeOpts::None)?;
        self.is_signed.encode(writer, &NetEncodeOpts::None)?;
        if self.is_signed {
            if let Some(sig) = &self.signature {
                sig.encode(writer, &NetEncodeOpts::None)?;
            } else {
                // signature claimed but missing; write empty string to be safe
                "".to_string().encode(writer, &NetEncodeOpts::None)?;
            }
        }
        Ok(())
    }

    async fn encode_async<W: tokio::io::AsyncWrite + Unpin>(
        &self,
        _writer: &mut W,
        _opts: &NetEncodeOpts,
    ) -> Result<(), NetEncodeError> {
        // Not used
        unimplemented!();
    }
}

/// Helper function to fully encode a PlayerInfo packet (action + players) into a writer.
pub fn encode_full_packet<W: Write>(writer: &mut W, kind: &PlayerInfoPacketKind) -> Result<(), NetEncodeError> {
    match kind {
        PlayerInfoPacketKind::Add(entries) => {
            // action 0, number of players
            VarInt::new(0).encode(writer, &NetEncodeOpts::None)?;
            VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
            for e in entries {
                // uuid as u128
                e.uuid.encode(writer, &NetEncodeOpts::None)?;
                // name
                e.name.encode(writer, &NetEncodeOpts::None)?;
                // properties
                e.properties.encode(writer, &NetEncodeOpts::None)?;
                // gamemode
                e.gamemode.encode(writer, &NetEncodeOpts::None)?;
                // ping
                e.ping.encode(writer, &NetEncodeOpts::None)?;
                // display_name (optional)
                match &e.display_name {
                    Some(text) => {
                        // There's no Option<T> encoding with length, encode a present flag?
                        // Protocol expects a boolean then component if present. We'll mimic that.
                        true.encode(writer, &NetEncodeOpts::None)?;
                        text.encode(writer, &NetEncodeOpts::None)?;
                    }
                    None => {
                        false.encode(writer, &NetEncodeOpts::None)?;
                    }
                }
            }
            Ok(())
        }
        PlayerInfoPacketKind::UpdateLatency(entries) => {
            VarInt::new(2).encode(writer, &NetEncodeOpts::None)?;
            VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
            for e in entries {
                e.uuid.encode(writer, &NetEncodeOpts::None)?;
                e.ping.encode(writer, &NetEncodeOpts::None)?;
            }
            Ok(())
        }
        PlayerInfoPacketKind::Remove(entries) => {
            VarInt::new(4).encode(writer, &NetEncodeOpts::None)?;
            VarInt::new(entries.len() as i32).encode(writer, &NetEncodeOpts::None)?;
            for e in entries {
                e.uuid.encode(writer, &NetEncodeOpts::None)?;
            }
            Ok(())
        }
    }
}
