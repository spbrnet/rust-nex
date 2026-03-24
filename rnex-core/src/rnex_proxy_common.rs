use crate::{PID, prudp::socket_addr::PRUDPSockAddr};
use macros::RmcSerialize;

#[derive(Debug, RmcSerialize)]
#[rmc_struct(0)]
pub struct ConnectionInitData {
    pub prudpsock_addr: PRUDPSockAddr,
    pub pid: PID,
}
