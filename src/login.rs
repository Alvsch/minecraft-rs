use std::net::{IpAddr, SocketAddr};

use evenio::prelude::*;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use valence_protocol::{anyhow::{self, ensure, Context}, ident, packets::login::{LoginCompressionS2c, LoginDisconnectS2c, LoginHelloC2s, LoginQueryRequestS2c, LoginQueryResponseC2s, LoginSuccessS2c}, profile::Property, text::{Color, IntoText}, uuid::Uuid, Decode, RawBytes, VarInt};

use crate::{block::BlockOn, client::Properties, packet_io::PacketIo, ClientLoginEvent, ConnectionMode, LoginEvent, Server};

#[derive(Debug, Clone)]
pub struct ClientInfo {
    pub username: String,
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub properties: Properties,
}

pub fn login_handler(r: ReceiverMut<LoginEvent>, server: Single<&Server>, mut sender: Sender<ClientLoginEvent>) {
    let event = EventMut::take(r.event);
    let mut packet_io = event.packet_io;

    let server = server.0;

    let info = async {
        if event.handshake.protocol_version != server.protocol_version {
            packet_io.send_packet(&LoginDisconnectS2c {
                // TODO: use correct translation key.
                reason: format!("Mismatched Minecraft version (server is on {})", server.version_name)
                    .color(Color::RED)
                    .into()
            }).await.unwrap();

            // Make sure client recieved disconnect packet
            packet_io.recv_frame().await.ok();

            return None;
        }

        let LoginHelloC2s {
            username,
            profile_id: _,
        } = packet_io.recv_packet().await.unwrap();

        let username = username.0.to_owned();

        let info = match &server.connection_mode {
            ConnectionMode::Online => login_online(&mut packet_io).await,
            ConnectionMode::Offline => login_offline(event.remote_ip, username).await,
            ConnectionMode::Velocity { secret } => login_velocity(&mut packet_io, username, &secret).await.unwrap(),
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

        Some(info)
    }.block();

    let Some(info) = info else { return; };

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
        properties: Properties::default()
    }
}

async fn login_velocity(
    packet_io: &mut PacketIo,
    username: String,
    velocity_secret: &str
) -> anyhow::Result<ClientInfo> {
    const VELOCITY_MIN_SUPPORTED_VERSION: u8 = 1;
    const VELOCITY_MODERN_FORWARDING_WITH_KEY_V2: i32 = 3;

    let message_id = 0;

    if packet_io.send_packet(&LoginQueryRequestS2c {
        message_id: VarInt(message_id),
        channel: ident!("velocity:player_info").into(),
        data: RawBytes(&[VELOCITY_MIN_SUPPORTED_VERSION]).into(),
    }).await.is_ok() {} ;

    let plugin_response: LoginQueryResponseC2s = packet_io.recv_packet().await.unwrap();

    ensure!(
        plugin_response.message_id.0 == message_id,
        "mismatched plugin response ID (got {}, expected {message_id})",
        plugin_response.message_id.0,
    );

    let data = plugin_response
        .data
        .context("missing plugin response data")?
        .0;

    ensure!(data.len() >= 32, "invalid plugin response data length");
    let (signature, mut data_without_signature) = data.split_at(32);

    // Verify signature
    let mut mac = Hmac::<Sha256>::new_from_slice(velocity_secret.as_bytes())?;
    Mac::update(&mut mac, data_without_signature);
    mac.verify_slice(signature)?;

    // Check Velocity version
    let version = VarInt::decode(&mut data_without_signature)
        .context("failed to decode velocity version")?
        .0;

    // Get client address
    let remote_addr = String::decode(&mut data_without_signature)?.parse()?;

    // Get UUID
    let uuid = Uuid::decode(&mut data_without_signature)?;

    // Get username and validate
    ensure!(
        username == <&str>::decode(&mut data_without_signature)?,
        "mismatched usernames"
    );

    // Read game profile properties
    let properties = Vec::<Property>::decode(&mut data_without_signature)
        .context("decoding velocity game profile properties")?;

    if version >= VELOCITY_MODERN_FORWARDING_WITH_KEY_V2 {
        // TODO
    }
    
    Ok(ClientInfo {
        username,
        uuid,
        ip: remote_addr,
        properties: Properties(properties)
    })
}