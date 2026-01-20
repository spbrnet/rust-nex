use crate::grpc::account;
use crate::reggie::{RemoteEdgeNodeHolder, RemoteEdgeNodeManagement};
use crate::{define_rmc_proto, kerberos};
use macros::rmc_struct;
use rnex_core::kerberos::{KerberosDateTime, Ticket, derive_key};
use rnex_core::nex::account::Account;
use rnex_core::rmc::protocols::OnlyRemote;
use rnex_core::rmc::protocols::auth::{Auth, RawAuth, RawAuthInfo, RemoteAuth};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::response::ErrorCode::Core_Unknown;
use rnex_core::rmc::structures::any::Any;
use rnex_core::rmc::structures::connection_data::ConnectionData;
use rnex_core::rmc::structures::qresult::QResult;
use std::hash::{DefaultHasher, Hasher};
use std::net::SocketAddrV4;
use std::sync::Arc;

define_rmc_proto!(
    proto AuthClientProtocol{
        Auth
    }
);

#[rmc_struct(AuthClientProtocol)]
pub struct AuthHandler {
    pub destination_server_acct: &'static Account,
    pub build_name: &'static str,
    //pub station_url: &'static str,
    pub control_server: Arc<OnlyRemote<RemoteEdgeNodeHolder>>,
}

pub fn generate_ticket(
    source_act_login_data: (u32, [u8; 16]),
    dest_act_login_data: (u32, [u8; 16]),
) -> Box<[u8]> {
    let source_key = derive_key(source_act_login_data.0, source_act_login_data.1);
    let dest_key = derive_key(dest_act_login_data.0, dest_act_login_data.1);

    let internal_data = kerberos::TicketInternalData::new(source_act_login_data.0);

    let encrypted_inner = internal_data.encrypt(dest_key);
    let encrypted_session_ticket = Ticket {
        pid: dest_act_login_data.0,
        session_key: internal_data.session_key,
    }
    .encrypt(source_key, &encrypted_inner);

    encrypted_session_ticket
}

async fn get_login_data_by_pid(pid: u32) -> Option<(u32, [u8; 16])> {
    let Ok(mut client) = account::Client::new().await else {
        return None;
    };

    let Ok(passwd) = client.get_nex_password(pid).await else {
        return None;
    };

    Some((pid, passwd))
}

fn station_url_from_sock_addr(sock_addr: SocketAddrV4) -> String {
    format!(
        "prudps:/PID=2;sid=1;stream=10;type=2;address={};port={};CID=1",
        sock_addr.ip(),
        sock_addr.port()
    )
}

impl Auth for AuthHandler {
    async fn login(&self, _name: String) -> Result<(), ErrorCode> {
        todo!()
    }

    async fn login_ex(
        &self,
        name: String,
        _extra_data: Any,
    ) -> Result<(QResult, u32, Vec<u8>, ConnectionData, String), ErrorCode> {
        let Ok(pid) = name.parse() else {
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Ok(mut client) = account::Client::new().await else {
            return Err(ErrorCode::Core_Exception);
        };

        let Ok(passwd) = client.get_nex_password(pid).await else {
            return Err(ErrorCode::Core_Exception);
        };

        let source_login_data = (pid, passwd);
        let destination_login_data = self.destination_server_acct.get_login_data();

        let ticket = generate_ticket(source_login_data, destination_login_data);

        let result = QResult::success(Core_Unknown);

        let mut hasher = DefaultHasher::new();

        hasher.write(name.as_bytes());

        let Ok(addr) = self.control_server.get_url(hasher.finish()).await else {
            return Err(ErrorCode::Core_Exception);
        };

        let connection_data = ConnectionData {
            station_url: station_url_from_sock_addr(addr),
            special_station_url: "".to_string(),
            //date_time: KerberosDateTime::new(1,1,1,1,1,1),
            date_time: KerberosDateTime::now(),
            special_protocols: Vec::new(),
        };

        Ok((
            result,
            source_login_data.0,
            ticket.into(),
            connection_data,
            self.build_name.to_string(), //format!("{}; Rust NEX Version {} by DJMrTV", self.build_name, env!("CARGO_PKG_VERSION")),
        ))
    }

    async fn request_ticket(
        &self,
        source_pid: u32,
        destination_pid: u32,
    ) -> Result<(QResult, Vec<u8>), ErrorCode> {
        let Some(source_login_data) = get_login_data_by_pid(source_pid).await else {
            return Err(ErrorCode::Core_Exception);
        };

        let desgination_login_data = if destination_pid == self.destination_server_acct.pid {
            self.destination_server_acct.get_login_data()
        } else {
            let Some(login) = get_login_data_by_pid(destination_pid).await else {
                return Err(ErrorCode::Core_Exception);
            };
            login
        };

        let result = QResult::success(Core_Unknown);

        let ticket = generate_ticket(source_login_data, desgination_login_data);

        Ok((result, ticket.into()))
    }

    async fn get_pid(&self, _username: String) -> Result<u32, ErrorCode> {
        Err(ErrorCode::Core_Exception)
    }

    async fn get_name(&self, _pid: u32) -> Result<String, ErrorCode> {
        Err(ErrorCode::Core_Exception)
    }
}

#[cfg(test)]
mod test {
    use rnex_core::rmc::response::RMCResponse;
    use rnex_core::rmc::structures::RmcSerialize;
    use rnex_core::rmc::structures::connection_data::ConnectionData;
    use rnex_core::rmc::structures::qresult::QResult;
    use std::io::Cursor;

    #[test]
    fn test() {
        return;
        let stuff = hex::decode("200100000a0106000000028000000100010051b3995774000000a6321c7f78847c1c5e9fb825eb26bd91841f1a40d92fc694159666119cb13527f1463ac48ad42a63e6613ede67041554b1770978112e6f1f3e177a2bfc75933216dbe38f70133a1eb28e2ae32a4b5c4b0c3e3efd4c02907992e259b257270b57a9dbe7792f4721b07f8fafb9e32d50f2555c616a015c0000004b007072756470733a2f5049443d323b7369643d313b73747265616d3d31303b747970653d323b616464726573733d322e3234332e39352e3131333b706f72743d31303030313b4349443d3100000000000100002c153ba51f00000033006272616e63683a6f726967696e2f70726f6a6563742f7775702d61676d6a206275696c643a335f385f31355f323030345f3000").unwrap();
        let stuff = RMCResponse::new(&mut Cursor::new(stuff)).unwrap();

        let rnex_core::rmc::response::RMCResponseResult::Success { .. } = stuff.response_result
        else {
            panic!()
        };

        // let stuff = hex::decode("0100010051B399577400000085F1736FCFBE93660275A3FE36FED6C2EFC57222AC99A9219CF54170A415B02DF1463AC48AD42A6307813FDE67041554B177097832ED000F892D9551A09F88E9CB0388DC1BC9527CC7384556A3287B2A349ABBF7E34A5A3EC14C2287CC7F78DA616BC3B03A035347FBD2E9A505C8EF42447CD809015F0000004E007072756470733A2F73747265616D3D31303B747970653D323B616464726573733D3139322E3136382E3137382E3132303B706F72743D31303030313B4349443D313B5049443D323B7369643D310000000000010000CDF53AA51F00000033006272616E63683A6F726967696E2F70726F6A6563742F7775702D61676D6A206275696C643A335F385F31355F323030345F3000").unwrap();
        let stuff = hex::decode("0100010051b399577400000037d3d4814d2b16dd546c94a75d32637b45f856b5abe73cf26cfaa235c5f2c1cef1463ac48ad42a637d873fde67041554b177097880cfa7e10bb810eaf686bfb0a0cf3d65b1f476ebc046d0855327986f557dca14fbb8594883c186b863f2206f22baa0309dbcc81da2f883cb2cdc12628ec7fced015c0000004b007072756470733a2f5049443d323b7369643d313b73747265616d3d31303b747970653d323b616464726573733d322e3234332e39352e3131333b706f72743d31303030313b4349443d310000000000010000b7f33aa51f00000033006272616e63683a6f726967696e2f70726f6a6563742f7775702d61676d6a206275696c643a335f385f31355f323030345f3000").unwrap();

        let data = <(QResult, u32, Vec<u8>, ConnectionData, String) as RmcSerialize>::deserialize(
            &mut Cursor::new(stuff),
        )
        .unwrap();

        println!("data: {:?}", data);
    }
}
