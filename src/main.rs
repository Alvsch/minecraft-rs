use std::sync::{atomic::AtomicUsize, Arc};

use client::{Client, IpAddress, Username};
use evenio::prelude::*;
use event::{ClientDisconnectEvent, ClientLoginEvent, ConnectionEvent, LoginEvent, StatusEvent};
use network::connect::handshake::connection_handler;
use network::connect::login::login_handler;
use status::status_handler;
use tokio::net::TcpListener;
use tracing::{info, Level};
use valence_protocol::CompressionThreshold;
use valence_server_common::UniqueId;

pub mod block;
pub mod status;
pub mod client;
pub mod event;
pub mod network;
pub mod position;
pub mod brand;

#[derive(Debug, Component)]
pub struct Server {
    version_name: String,
    protocol_version: i32,
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
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    let mut world = World::new();
    world.add_handler(connection_handler);
    world.add_handler(status_handler);
    world.add_handler(login_handler);
    world.add_handler(client_login_handler);
    world.add_handler(client_disconnect_handler);
    world.add_handler(init_client);

    // world.add_plugin(HitboxPlugin);
    // world.add_plugin(EntityPlugin);


    let server = world.spawn();
    world.insert(server, Server {
        version_name: "1.20.1".to_string(),
        protocol_version: 763,
        max_players: 20,
        online: AtomicUsize::new(0),
        motd: "A Valence Minecraft Server".to_string(),
        favicon: "".to_string(),
        connection_mode: ConnectionMode::Offline,
        threshold: CompressionThreshold(256),
    });

    let Ok(listener) = TcpListener::bind("127.0.0.1:25566").await else { return; };

    info!("Listening for connections");
    loop  {
        let Ok((stream, remote_addr)) = listener.accept().await else { continue; };
        world.send(ConnectionEvent {
            stream,
            remote_addr,
        });
    }
}

fn client_login_handler(
    r: ReceiverMut<ClientLoginEvent>, 
    mut sender: Sender<(
        Spawn,
        Insert<Client>,
        Insert<IpAddress>,
        Insert<Username>,
        Insert<UniqueId>,
    )>
) {
    let event = EventMut::take(r.event);
    let packet_io = event.packet_io;
    let info = event.info;

    let client = sender.spawn();
    sender.insert(client, packet_io.into_client());
    sender.insert(client, IpAddress(info.ip));
    sender.insert(client, Username(info.username));
    sender.insert(client, UniqueId(info.uuid));

}

fn init_client(r: Receiver<Insert<Client>, ()>) {
    let entity = r.event.entity;

    

}

fn client_disconnect_handler(r: Receiver<ClientDisconnectEvent>, mut sender: Sender<Despawn>) {
    let entity = r.event.entity;
    sender.despawn(entity);
}
