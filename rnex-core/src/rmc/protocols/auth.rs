use crate::rmc::structures::connection_data::{ConnectionData, ConnectionDataOld};
use cfg_if::cfg_if;
use macros::{method_id, rmc_proto};
use rnex_core::PID;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::any::Any;
use rnex_core::rmc::structures::qresult::QResult;

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

    #[method_id(3)]
    async fn request_ticket(
        &self,
        source_pid: PID,
        destination_pid: PID,
    ) -> Result<RequestTicketRet, ErrorCode>;

    /// representation of the `RequestTicket` method(for details see the
    /// [kinnay wiki entry](https://github.com/kinnay/NintendoClients/wiki/Authentication-Protocol))

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
