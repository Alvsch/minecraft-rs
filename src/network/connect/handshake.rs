use std::io;

use evenio::prelude::*;
use tracing::warn;
use valence_protocol::{packets::handshaking::{handshake_c2s::HandshakeNextState, HandshakeC2s}, PacketDecoder, PacketEncoder};

use crate::{block::BlockOn, event::{ConnectionEvent, LoginEvent, StatusEvent}, network::connect::legacy_ping::try_handle_legacy_ping, network::packet_io::PacketIo};

#[derive(Debug, Clone)]
pub struct HandshakeData {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
}

impl<'a> From<HandshakeC2s<'a>> for HandshakeData {
    fn from(value: HandshakeC2s) -> Self {
        HandshakeData {
            protocol_version: value.protocol_version.0,
            server_address: value.server_address.to_string(),
            server_port: value.server_port,
        }
    }
}

pub fn connection_handler(r: ReceiverMut<ConnectionEvent>, mut sender: Sender<(StatusEvent, LoginEvent)>) {
    let mut event = EventMut::take(r.event);

    async {
        match try_handle_legacy_ping(&mut event.stream).await {
            Ok(true) => return, // Legacy ping succeeded.
            Ok(false) => {}     // No legacy ping.
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {}
            Err(e) => {
                warn!("legacy ping ended with error: {e:#}");
            }
        }

        let mut packet_io = PacketIo::new(
            event.stream,
            PacketEncoder::new(),
            PacketDecoder::new(),
        );

        let Ok(handshake) = packet_io.recv_packet::<HandshakeC2s>().await else { return; };
        match handshake.next_state {
            HandshakeNextState::Status => {
                sender.send(StatusEvent {
                    packet_io,
                });
            }
            HandshakeNextState::Login => {
                sender.send(LoginEvent {
                    handshake: handshake.into(),
                    packet_io,
                    remote_ip: event.remote_addr,
                });
            },
        }
    }.block();
}