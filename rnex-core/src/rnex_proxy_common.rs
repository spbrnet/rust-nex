use macros::RmcSerialize;
use crate::prudp::socket_addr::PRUDPSockAddr;

#[derive(Debug, RmcSerialize)]
#[rmc_struct(0)]
pub struct ConnectionInitData{
    pub prudpsock_addr: PRUDPSockAddr,
    pub pid: u32,
    
}

