use crate::{PID, prudp::socket_addr::PRUDPSockAddr};
use macros::RmcSerialize;

#[derive(Debug, RmcSerialize, PartialEq, Eq)]
#[rmc_struct(0)]
pub struct ConnectionInitData {
    pub prudpsock_addr: PRUDPSockAddr,
    pub pid: PID,
}

#[cfg(test)]
mod test {
    use std::{
        io::Cursor,
        net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    };

    use crate::{
        prudp::{socket_addr::PRUDPSockAddr, virtual_port::VirtualPort},
        rmc::structures::RmcSerialize,
        rnex_proxy_common::ConnectionInitData,
    };

    #[test]
    fn test() {
        let data = ConnectionInitData {
            prudpsock_addr: PRUDPSockAddr {
                regular_socket_addr: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::BROADCAST, 19293)),
                virtual_port: VirtualPort::new(10, 2),
            },
            pid: 100,
        };

        let ser = data.to_data().unwrap();

        let de = ConnectionInitData::deserialize(&mut Cursor::new(ser)).unwrap();

        assert_eq!(data, de);
    }
}
