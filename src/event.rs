use std::net::SocketAddr;

use evenio::{entity::EntityId, event::Event};
use tokio::net::TcpStream;

use crate::{network::connect::handshake::HandshakeData, network::connect::login::ClientInfo, network::packet_io::PacketIo};

#[derive(Debug, Event)]
pub struct ConnectionEvent {
    pub stream: TcpStream,
    pub remote_addr: SocketAddr,
}

#[derive(Debug, Event)]
pub struct ClientDisconnectEvent {
    pub entity: EntityId,
}

#[derive(Event)]
pub struct StatusEvent {
    pub packet_io: PacketIo,
}

#[derive(Event)]
pub struct LegacyPingEvent {
    pub packet_io: PacketIo,
}

#[derive(Event)]
pub struct LoginEvent {
    pub handshake: HandshakeData,
    pub packet_io: PacketIo,
    pub remote_ip: SocketAddr,
}

#[derive(Event)]
pub struct ClientLoginEvent {
    pub packet_io: PacketIo,
    pub info: ClientInfo,
}