use futures_util::{SinkExt, StreamExt};
use rnex_core::prudp::types_flags::{TypesFlags, flags::NEED_ACK, types::SYN};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest, http::header};

use crate::packet::{LiteHeader, LitePacket, PacketSpecificData, StreamTypes, create_packet_from};

mod packet;

const KEY: &str = "4eb18d39";

const URL: &str = "wss://g2DF33D01-lp1.s.n.srv.nintendo.net";
#[tokio::main]
async fn main() {
    let login = URL.into_client_request().unwrap();
    let (mut stream, response) = tokio_tungstenite::connect_async(login).await.unwrap();

    println!("response: {:?}", response);

    let packet = create_packet_from(
        LiteHeader {
            stream_types: StreamTypes::new(10, 10),
            source_port: 1,
            destination_port: 1,
            fragment_id: 0,
            types_flags: TypesFlags::default().types(SYN).flags(NEED_ACK),
            sequence_id: 0,
            ..Default::default()
        },
        &[PacketSpecificData::SupportedFunctions(0x8)],
        &[],
    );

    println!("sending ack");
    stream.send(Message::Binary(packet.into())).await.unwrap();
    println!("waiting for response");
    let packet = stream.next().await.unwrap();
    let Message::Binary(packet) = packet.unwrap() else {
        panic!()
    };
    let packet = LitePacket::new(packet);

    let header = packet.header().unwrap();

    println!("{:?}", header);
}
