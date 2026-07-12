use cfg_if::cfg_if;
use rnex_rmc::{
    RmcSerialize,
    any::Any,
    data::Data,
    method_id,
    qresult::QResult,
    response::ErrorCode,
    rmc_proto,
    util::{PID, date_time::DateTime},
};

#[derive(Debug, RmcSerialize)]
#[rmc_struct(1)]
pub struct ConnectionData {
    pub station_url: String,
    pub special_protocols: Vec<u8>,
    pub special_station_url: String,
    pub date_time: DateTime,
}

#[derive(Debug, RmcSerialize)]
#[rmc_struct(1)]
pub struct ConnectionDataOld {
    pub station_url: String,
    pub special_protocols: Vec<u8>,
    pub special_station_url: String,
}

cfg_if! {
    if #[cfg(feature = "nx")]{
        type LoginExRet = (QResult, PID, Vec<u8>, ConnectionData, String, String);
        type RequestTicketRet = (QResult, Vec<u8>, String);
    } else {
        type LoginExRet = (QResult, PID, Vec<u8>, ConnectionData, String);
        type RequestTicketRet = (QResult, Vec<u8>);
    }
}

/// This is the representation for `Ticket Granting`(for details see the
/// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
#[rmc_proto(10)]
pub trait Auth {
    /// representation of the `Login` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
    #[method_id(1)]
    async fn login(
        &self,
        name: String,
    ) -> Result<(QResult, PID, Vec<u8>, ConnectionDataOld, String), ErrorCode>;

    /// representation of the `LoginEx` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
    #[method_id(2)]
    async fn login_ex(&self, name: String, extra_data: Any) -> Result<LoginExRet, ErrorCode>;

    /// representation of the `RequestTicket` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
    #[method_id(3)]
    async fn request_ticket(
        &self,
        source_pid: PID,
        destination_pid: PID,
    ) -> Result<RequestTicketRet, ErrorCode>;

    /// representation of the `GetPID` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
    #[method_id(4)]
    async fn get_pid(&self, username: String) -> Result<u32, ErrorCode>;

    /// representation of the `LoginWithContext` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))
    #[method_id(5)]
    async fn get_name(&self, pid: PID) -> Result<String, ErrorCode>;

    // `LoginWithContext` is left out here because we don't need it right now and versioning still
    // needs to be figured out
}

#[derive(RmcSerialize)]
#[rmc_struct(0)]
struct AuthenticationInfo {
    #[extends]
    pub data: Data,
    pub auth_token: String,
    pub ngs_version: u32,
    pub auth_token_type: u8,
    pub server_version: u32,
}
