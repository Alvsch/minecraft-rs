use std::net::{IpAddr, SocketAddr};

use evenio::prelude::*;
use valence_protocol::{packets::login::{LoginCompressionS2c, LoginDisconnectS2c, LoginHelloC2s, LoginSuccessS2c}, text::{Color, IntoText}, uuid::Uuid};

use crate::{block::BlockOn, packet_io::PacketIo, Client, ClientLoginEvent, ConnectionMode, LoginEvent, Server};

pub struct ClientInfo {
    username: String,
    uuid: Uuid,
    ip: IpAddr,
}

pub fn login_handler(r: ReceiverMut<LoginEvent>, server: Single<&Server>, mut sender: Sender<ClientLoginEvent>) {
    let event = EventMut::take(r.event);
    let mut packet_io = event.packet_io;

    let server = server.0;

    let info = async {
        if /* handshake.protocol_version != PROTOCOL_VERSION */ false {
            packet_io.send_packet(&LoginDisconnectS2c {
                // TODO: use correct translation key.
                reason: format!("Mismatched Minecraft version (server is on {})", server.version_name)
                    .color(Color::RED)
                    .into()
            }).await.unwrap();
        }

        let LoginHelloC2s {
            username,
            profile_id: _,
        } = packet_io.recv_packet().await.unwrap();

        let username = username.0.to_owned();

        let info = match &server.connection_mode {
            ConnectionMode::Online => login_online(&mut packet_io).await,
            ConnectionMode::Offline => login_offline(event.remote_ip, username).await,
            ConnectionMode::Velocity { secret } => login_velocity(&mut packet_io).await,
        };

        if server.threshold.0 > 0 {
            packet_io.send_packet(&LoginCompressionS2c {
                threshold: server.threshold.0.into(),
            }).await.unwrap();

            packet_io.set_compression(server.threshold);
        }

        packet_io.send_packet(&LoginSuccessS2c {
            uuid: info.uuid,
            username: info.username.as_str().into(),
            properties: Default::default(),
        }).await.unwrap();

        info
    }.block();

    sender.send(ClientLoginEvent {
        packet_io,
        info,
    });
}

async fn login_online(packet_io: &mut PacketIo) -> ClientInfo {
    todo!()
}

fn offline_uuid(username: &str) -> Uuid {
    let hash = md5::compute(format!("OfflinePlayer:{}", username));
    Uuid::from_slice(&hash[..16]).unwrap()
}

async fn login_offline(remote_ip: SocketAddr, username: String) -> ClientInfo {
    ClientInfo {
        uuid: offline_uuid(&username),
        username,
        ip: remote_ip.ip(),
    }
}

async fn login_velocity(packet_io: &mut PacketIo) -> ClientInfo {
    todo!()
}