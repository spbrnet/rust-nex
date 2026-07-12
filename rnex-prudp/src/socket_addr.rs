use hmac::Hmac;
use md5::digest::Mac;
use rc4::KeyInit;
use std::net::{IpAddr, SocketAddr};

use crate::virtual_port::VirtualPort;

type Md5Hmac = Hmac<md5::Md5>;

#[derive(Eq, PartialEq, Hash, Debug, Copy, Clone, Ord, PartialOrd)]
pub struct PRUDPSockAddr {
    pub regular_socket_addr: SocketAddr,
    pub virtual_port: VirtualPort,
}

impl PRUDPSockAddr {
    pub fn new(regular_socket_addr: SocketAddr, virtual_port: VirtualPort) -> Self {
        Self {
            regular_socket_addr,
            virtual_port,
        }
    }

    pub fn calculate_connection_signature(&self) -> [u8; 16] {
        let mut hmac = Md5Hmac::new_from_slice(&[0; 16]).expect("?");

        let data = match self.regular_socket_addr.ip() {
            IpAddr::V4(v) => v.octets().to_vec(),
            IpAddr::V6(v) => v.octets().to_vec(),
        };
        //data.extend_from_slice(&self.regular_socket_addr.port().to_be_bytes());

        hmac.update(&data);
        let result: [u8; 16] = hmac.finalize().into_bytes()[0..16]
            .try_into()
            .expect("fuck");
        result
    }
}
