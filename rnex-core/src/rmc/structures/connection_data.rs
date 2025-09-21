
use macros::RmcSerialize;
use rnex_core::kerberos::KerberosDateTime;

#[derive(Debug, RmcSerialize)]
#[rmc_struct(1)]
pub struct ConnectionData{
    pub station_url: String,
    pub special_protocols: Vec<u8>,
    pub special_station_url: String,
    pub date_time: KerberosDateTime
}

