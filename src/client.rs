use std::{borrow::Cow, net::IpAddr};
use derive_more::{Deref, DerefMut};

use evenio::component::Component;
use tokio::{io::AsyncWriteExt, net::TcpStream};
use valence_entity::{EntityStatus, Velocity};
use valence_protocol::{anyhow, math::{DVec3, Vec3}, packets::play::{game_state_change_s2c::GameEventKind, DeathMessageS2c, EntityStatusS2c, EntityVelocityUpdateS2c, GameStateChangeS2c, ParticleS2c, PlaySoundS2c}, profile::Property, sound::{SoundCategory, SoundId}, text::IntoText, BlockPos, Encode, GameMode, Ident, Packet, PacketEncoder, Particle, Sound, VarInt, WritePacket};

use crate::block::BlockOn;

#[derive(Component)]
pub struct Client {
    pub stream: TcpStream,
    pub enc: PacketEncoder,
}

/// Writes packets into this client's packet buffer. The buffer is flushed at
/// the end of the tick.
impl WritePacket for Client {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.enc.write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.enc.write_packet_bytes(bytes)
    }
}

impl Client {
    pub fn connection(&self) -> &TcpStream {
        &self.stream
    }

    pub fn connection_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Flushes the packet queue to the underlying connection.
    ///
    /// This is called automatically at the end of the tick and when the client
    /// is dropped. Unless you're in a hurry, there's usually no reason to
    /// call this method yourself.
    ///
    /// Returns an error if flushing was unsuccessful.
    pub fn flush_packets(&mut self) -> anyhow::Result<()> {
        let bytes = self.enc.take();
        if !bytes.is_empty() {
            self.stream.write_all(&bytes).block()?;
        }
        Ok(())
    }

    /// Kills the client and shows `message` on the death screen. If an entity
    /// killed the player, you should supply it as `killer`.
    pub fn kill<'a>(&mut self, message: impl IntoText<'a>) {
        self.write_packet(&DeathMessageS2c {
            player_id: VarInt(0),
            message: message.into_cow_text(),
        });
    }

    /// Respawns client. Optionally can roll the credits before respawning.
    pub fn win_game(&mut self, show_credits: bool) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::WinGame,
            value: if show_credits { 1.0 } else { 0.0 },
        });
    }

    /// Puts a particle effect at the given position, only for this client.
    pub fn play_particle(
        &mut self,
        particle: &Particle,
        long_distance: bool,
        position: impl Into<DVec3>,
        offset: impl Into<Vec3>,
        max_speed: f32,
        count: i32,
    ) {
        self.write_packet(&ParticleS2c {
            particle: Cow::Borrowed(particle),
            long_distance,
            position: position.into(),
            offset: offset.into(),
            max_speed,
            count,
        })
    }

    /// Plays a sound effect at the given position, only for this client.
    pub fn play_sound(
        &mut self,
        sound: Sound,
        category: SoundCategory,
        position: impl Into<DVec3>,
        volume: f32,
        pitch: f32,
    ) {
        let position = position.into();

        self.write_packet(&PlaySoundS2c {
            id: SoundId::Direct {
                id: sound.to_ident().into(),
                range: None,
            },
            category,
            position: (position * 8.0).as_ivec3(),
            volume,
            pitch,
            seed: rand::random(),
        });
    }

    /// `velocity` is in m/s.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3>) {
        self.write_packet(&EntityVelocityUpdateS2c {
            entity_id: VarInt(0),
            velocity: Velocity(velocity.into()).to_packet_units(),
        });
    }

    /// Triggers an [`EntityStatus`].
    ///
    /// The status is only visible to this client.
    pub fn trigger_status(&mut self, status: EntityStatus) {
        self.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: status as u8,
        });
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Deref, DerefMut)]
pub struct IpAddress(pub IpAddr);

#[derive(Component, Clone, PartialEq, Eq, Debug, Deref, DerefMut)]
pub struct Username(pub String);

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct DeathLocation(pub Option<(Ident<String>, BlockPos)>);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsHardcore(pub bool);

/// Hashed world seed used for biome noise.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct HashedSeed(pub u64);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct ReducedDebugInfo(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Deref, DerefMut)]
pub struct HasRespawnScreen(pub bool);

/// If the client is spawning into a debug world.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsDebug(pub bool);

/// Changes the perceived horizon line (used for superflat worlds).
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsFlat(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct PortalCooldown(pub i32);

/// The initial previous gamemode. Used for the F3+F4 gamemode switcher.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
   pub struct PrevGameMode(pub Option<GameMode>);

impl Default for HasRespawnScreen {
    fn default() -> Self {
        Self(true)
    }
}

/// The position and angle that clients will respawn with. Also
/// controls the position that compasses point towards.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct RespawnPosition {
    /// The position that clients will respawn at. This can be changed at any
    /// time to set the position that compasses point towards.
    pub pos: BlockPos,
    /// The yaw angle that clients will respawn with (in degrees).
    pub yaw: f32,
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct Properties(pub Vec<Property>);
