use std::sync::atomic::Ordering;

use evenio::prelude::*;
use serde_json::json;
use valence_protocol::packets::status::{QueryPingC2s, QueryPongS2c, QueryRequestC2s, QueryResponseS2c};

use crate::{block::BlockOn, Server, StatusEvent};

pub fn status_handler(r: ReceiverMut<StatusEvent>, server: Single<&Server>) {
    let mut packet_io = EventMut::take(r.event).packet_io;
    let server = server.0;

    async {
        packet_io.recv_packet::<QueryRequestC2s>().await.unwrap();

        let json = json!({
            "version": {
                "name": server.version_name,
                "protocol": server.protocol_id,
            },
            "players": {
                "max": server.max_players,
                "online": server.online.load(Ordering::Relaxed),
                "sample": [],
            },
            "description": {
                "text": server.motd,
            },
            "favicon": server.favicon,
            "enforcesSecureChat": true,
            "previewsChat": true,
        }).to_string();
    
        packet_io.send_packet(&QueryResponseS2c {
            json: &json,
        }).await.unwrap();
    
        let ping = packet_io.recv_packet::<QueryPingC2s>().await.unwrap();
        packet_io.send_packet(&QueryPongS2c {
            payload: ping.payload,
        }).await.unwrap();
    }.block();
}