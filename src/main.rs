use std::{net::SocketAddr, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use block::BlockOn;
use evenio::prelude::*;
use login::{login_handler, ClientInfo};
use packet_io::PacketIo;
use serde_json::json;
use status::status_handler;
use tokio::net::{TcpListener, TcpStream};
use valence_protocol::{packets::{handshaking::{handshake_c2s::HandshakeNextState, HandshakeC2s}, status::{QueryPingC2s, QueryPongS2c, QueryRequestC2s, QueryResponseS2c}}, CompressionThreshold, PacketDecoder, PacketEncoder};

pub mod packet_io;
pub mod block;
pub mod login;
pub mod status;

#[derive(Component)]
pub struct Client {
    pub address: SocketAddr,
}

#[derive(Debug, Event)]
pub struct ConnectionEvent {
    pub stream: TcpStream,
    pub remote_ip: SocketAddr,
}

#[derive(Event)]
pub struct StatusEvent {
    packet_io: PacketIo,
}

#[derive(Event)]
pub struct LoginEvent {
    packet_io: PacketIo,
    remote_ip: SocketAddr,
}

#[derive(Event)]
pub struct ClientLoginEvent {
    pub packet_io: PacketIo,
    pub info: ClientInfo,
}

#[derive(Debug, Component)]
pub struct Server {
    version_name: String,
    protocol_id: i32,
    max_players: usize,
    online: AtomicUsize,
    motd: String,
    favicon: String,
    connection_mode: ConnectionMode,
    threshold: CompressionThreshold,
}

#[derive(Debug, Default)]
pub enum ConnectionMode {
    #[default]
    Online,
    Offline,
    Velocity {
        secret: Arc<str>,
    }
}

#[tokio::main]
async fn main() {
    let mut world = World::new();
    world.add_handler(connection_handler);
    world.add_handler(status_handler);
    world.add_handler(login_handler);

    let server = world.spawn();
    world.insert(server, Server {
        version_name: "1.20.1".to_string(),
        protocol_id: 763,
        max_players: 20,
        online: AtomicUsize::new(0),
        motd: "A Valence Minecraft Server".to_string(),
        favicon: "".to_string(),
        connection_mode: ConnectionMode::Offline,
        threshold: CompressionThreshold(256),
    });
    
    let listener = TcpListener::bind("127.0.0.1:25566").await.unwrap();
    loop  {
        let Ok((stream, address)) = listener.accept().await else { continue; };
        world.send(ConnectionEvent {
            stream,
            remote_ip: address
        });
    }
}

fn client_login_handler(r: ReceiverMut<ClientLoginEvent>) {
    let event = EventMut::take(r.event);
    let mut packet_io = event.packet_io;

    let packet = packet_io.recv_packet();
}

fn connection_handler(r: ReceiverMut<ConnectionEvent>, mut sender: Sender<(StatusEvent, LoginEvent)>) {
    let event = EventMut::take(r.event);

    let mut packet_io = PacketIo::new(
        event.stream,
        PacketEncoder::new(),
        PacketDecoder::new(),
    );

    async {
        let handshake = packet_io.recv_packet::<HandshakeC2s>().await.unwrap();
        match handshake.next_state {
            HandshakeNextState::Status => {
                sender.send(StatusEvent {
                    packet_io,
                });
            }
            HandshakeNextState::Login => {
                sender.send(LoginEvent {
                    packet_io,
                    remote_ip: event.remote_ip,
                });
            },
        }

    }.block();
}
