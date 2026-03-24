use crate::grpc::account;
use crate::reggie::{RemoteEdgeNodeHolder, RemoteEdgeNodeManagement};
use crate::{define_rmc_proto, kerberos};
use cfg_if::cfg_if;
use log::{info, warn};
use macros::rmc_struct;
use rnex_core::PID;
use rnex_core::kerberos::{KerberosDateTime, Ticket, derive_key};
use rnex_core::nex::account::Account;
use rnex_core::rmc::protocols::OnlyRemote;
use rnex_core::rmc::protocols::auth::{Auth, RawAuth, RawAuthInfo, RemoteAuth};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::response::ErrorCode::Core_Unknown;
use rnex_core::rmc::structures::any::Any;
use rnex_core::rmc::structures::connection_data::ConnectionData;
use rnex_core::rmc::structures::connection_data::ConnectionDataOld;
use rnex_core::rmc::structures::qresult::QResult;
use std::hash::{DefaultHasher, Hasher};
use std::net::SocketAddrV4;
use std::sync::{Arc, LazyLock};

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
    source_act_login_data: (PID, &[u8]),
    dest_act_login_data: (PID, &[u8]),
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
pub fn generate_ticket_with_string_user_key(
    source_act: PID,
    dest_act_login_data: (PID, &[u8]),
) -> (String, Box<[u8]>) {
    let source_key: [u8; 8] = rand::random();
    let key_string = hex::encode(source_key);
    let key_data: [u8; 16] = key_string.as_bytes().try_into().unwrap();
    let dest_key = derive_key(dest_act_login_data.0, dest_act_login_data.1);

    let internal_data = kerberos::TicketInternalData::new(source_act);

    let encrypted_inner = internal_data.encrypt(dest_key);
    let encrypted_session_ticket = Ticket {
        pid: dest_act_login_data.0,
        session_key: internal_data.session_key,
    }
    .encrypt(key_data, &encrypted_inner);

    (key_string, encrypted_session_ticket)
}

async fn get_login_data_by_pid(pid: PID) -> Option<(PID, Box<[u8]>)> {
    if pid == GUEST_ACCOUNT.pid {
        let source_login_data = GUEST_ACCOUNT.get_login_data();

        return Some((source_login_data.0, source_login_data.1.into()));
    }

    let Ok(mut client) = account::Client::new().await else {
        return None;
    };

    let Ok(passwd) = client.get_nex_password(pid).await else {
        return None;
    };

    Some((pid, passwd.into()))
}

fn station_url_from_sock_addr(sock_addr: SocketAddrV4) -> String {
    format!(
        "prudps:/PID=2;sid=1;stream=10;type=2;address={};port={};CID=1",
        sock_addr.ip(),
        sock_addr.port()
    )
}

static GUEST_ACCOUNT: LazyLock<Account> =
    LazyLock::new(|| Account::new(100, "guest", "MMQea3n!fsik"));

impl AuthHandler {
    pub async fn generate_ticket_from_name(
        &self,
        name: &str,
    ) -> Result<(PID, Box<[u8]>), ErrorCode> {
        #[cfg(feature = "guest_login")]
        {
            if name == GUEST_ACCOUNT.username {
                let source_login_data = GUEST_ACCOUNT.get_login_data();
                let destination_login_data = self.destination_server_acct.get_login_data();

                return Ok((
                    source_login_data.0,
                    generate_ticket(source_login_data, destination_login_data),
                ));
            }
        }
        let Ok(pid) = name.parse() else {
            warn!("unable to connect to parse pid: {}", name);
            return Err(ErrorCode::Core_InvalidArgument);
        };

        let Ok(mut client) = account::Client::new().await else {
            warn!("unable to connect to grpc");
            return Err(ErrorCode::Core_Exception);
        };

        let Ok(passwd) = client.get_nex_password(pid).await else {
            warn!("unable to get nex password");
            return Err(ErrorCode::Core_Exception);
        };

        let source_login_data = (pid, &passwd[..]);
        println!("{}, {:?}", pid, passwd);
        let destination_login_data = self.destination_server_acct.get_login_data();

        Ok((
            pid,
            generate_ticket(source_login_data, destination_login_data),
        ))
    }

    pub async fn generate_ticket_from_name_string_user_key(
        &self,
        name: &str,
    ) -> Result<(PID, String, Box<[u8]>), ErrorCode> {
        {
            if name == GUEST_ACCOUNT.username {
                let source_login_data = GUEST_ACCOUNT.get_login_data();
                let destination_login_data = self.destination_server_acct.get_login_data();
                let ticket = generate_ticket_with_string_user_key(
                    source_login_data.0,
                    destination_login_data,
                );

                return Ok((source_login_data.0, ticket.0, ticket.1));
            }
        }
        let Ok(pid) = name.parse() else {
            warn!("unable to connect to parse pid: {}", name);
            return Err(ErrorCode::Core_InvalidArgument);
        };
        let destination_login_data = self.destination_server_acct.get_login_data();

        let data = generate_ticket_with_string_user_key(pid, destination_login_data);
        Ok((pid, data.0, data.1))
    }
}

impl Auth for AuthHandler {
    async fn login(
        &self,
        name: String,
    ) -> Result<(QResult, PID, Vec<u8>, ConnectionDataOld, String), ErrorCode> {
        let (pid, ticket) = self.generate_ticket_from_name(&name).await?;

        let result = QResult::success(Core_Unknown);

        let mut hasher = DefaultHasher::new();

        hasher.write(name.as_bytes());

        let Ok(addr) = self.control_server.get_url(hasher.finish()).await else {
            warn!("no secure proxies");
            return Err(ErrorCode::Core_Exception);
        };

        let connection_data = ConnectionDataOld {
            station_url: station_url_from_sock_addr(addr),
            special_station_url: "".to_string(),
            special_protocols: Vec::new(),
        };

        let ret = (
            result,
            pid,
            ticket.into(),
            connection_data,
            self.build_name.to_string(),
        );

        info!("data: {:?}", ret);
        Ok(ret)
    }
    cfg_if! {

        if #[cfg(feature = "nx")]{
            async fn login_ex(
                &self,
                name: String,
                _extra_data: Any,
            ) -> Result<(QResult, PID, Vec<u8>, ConnectionData, String, String), ErrorCode> {
                let (pid, key, ticket) = self.generate_ticket_from_name_string_user_key(&name).await?;

                let result = QResult::success(Core_Unknown);

                let mut hasher = DefaultHasher::new();

                hasher.write(name.as_bytes());

                let Ok(addr) = self.control_server.get_url(hasher.finish()).await else {
                    warn!("no secure proxies");
                    return Err(ErrorCode::Core_Exception);
                };

                let connection_data = ConnectionData {
                    station_url: station_url_from_sock_addr(addr),
                    special_station_url: "".to_string(),
                    //date_time: KerberosDateTime::new(1,1,1,1,1,1),
                    date_time: KerberosDateTime::now(),
                    special_protocols: Vec::new(),
                };

                let ret = (
                    result,
                    pid,
                    ticket.into(),
                    connection_data,
                    self.build_name.to_string(),
                    key
                );

                info!("data: {:?}", ret);
                Ok(ret)
            }
            async fn request_ticket(
                &self,
                source_pid: PID,
                destination_pid: PID,
            ) -> Result<(QResult, Vec<u8>, String), ErrorCode> {
                let Some((pid, _)) = get_login_data_by_pid(source_pid).await else {
                    return Err(ErrorCode::Core_Exception);
                };

                let desgination_login_data = if destination_pid == self.destination_server_acct.pid {
                    self.destination_server_acct.get_login_data()
                } else {
                    return Err(ErrorCode::RendezVous_InvalidOperation);
                };

                let result = QResult::success(Core_Unknown);

                let ticket = generate_ticket_with_string_user_key(pid, desgination_login_data);

                Ok((result, ticket.1.into(), ticket.0))
            }
        } else {
            async fn login_ex(
                &self,
                name: String,
                _extra_data: Any,
            ) -> Result<(QResult, PID, Vec<u8>, ConnectionData, String), ErrorCode> {
                let (pid, ticket) = self.generate_ticket_from_name(&name).await?;

                let result = QResult::success(Core_Unknown);

                let mut hasher = DefaultHasher::new();

                hasher.write(name.as_bytes());

                let Ok(addr) = self.control_server.get_url(hasher.finish()).await else {
                    warn!("no secure proxies");
                    return Err(ErrorCode::Core_Exception);
                };

                let connection_data = ConnectionData {
                    station_url: station_url_from_sock_addr(addr),
                    special_station_url: "".to_string(),
                    //date_time: KerberosDateTime::new(1,1,1,1,1,1),
                    date_time: KerberosDateTime::now(),
                    special_protocols: Vec::new(),
                };

                let ret = (
                    result,
                    pid,
                    ticket.into(),
                    connection_data,
                    self.build_name.to_string(),
                );

                info!("data: {:?}", ret);
                Ok(ret)
            }
            async fn request_ticket(
                &self,
                source_pid: PID,
                destination_pid: PID,
            ) -> Result<(QResult, Vec<u8>), ErrorCode> {
                let Some((pid, passwd)) = get_login_data_by_pid(source_pid).await else {
                    return Err(ErrorCode::Core_Exception);
                };

                let desgination_login_data = if destination_pid == self.destination_server_acct.pid {
                    self.destination_server_acct.get_login_data()
                } else {
                    return Err(ErrorCode::RendezVous_InvalidOperation);
                };

                let result = QResult::success(Core_Unknown);

                let ticket = generate_ticket((pid, &passwd[..]), desgination_login_data);

                Ok((result, ticket.into()))
            }
        }
    }

    async fn get_pid(&self, _username: String) -> Result<u32, ErrorCode> {
        Err(ErrorCode::Core_Exception)
    }

    async fn get_name(&self, _pid: PID) -> Result<String, ErrorCode> {
        Err(ErrorCode::Core_Exception)
    }
}

mod test {
    use std::io::Cursor;

    use rc4::{KeyInit, Rc4, StreamCipher};
    use rnex_core::PID;
    use rnex_core::kerberos::KerberosDateTime;
    use rnex_core::rmc::structures::connection_data::ConnectionData;
    use rnex_core::rmc::{
        response::ErrorCode,
        structures::{RmcSerialize, qresult::QResult},
    };

    use crate::kerberos::{self, derive_key};
    use crate::rmc;
    use crate::rmc::message::RMCMessage;
    use crate::rmc::response::{RMCResponse, RMCResponseResult};

    #[test]
    fn test() {
        return;
        let packet = [
            26, 1, 0, 0, 10, 1, 30, 0, 0, 0, 1, 128, 0, 0, 1, 0, 1, 0, 86, 4, 0, 0, 116, 0, 0, 0,
            144, 209, 130, 175, 45, 215, 95, 55, 226, 192, 51, 54, 201, 84, 118, 150, 159, 164, 32,
            103, 134, 252, 199, 168, 178, 5, 6, 208, 206, 241, 94, 23, 136, 37, 109, 247, 156, 252,
            189, 233, 142, 115, 206, 72, 180, 57, 106, 223, 37, 59, 144, 208, 250, 197, 51, 202,
            185, 156, 51, 159, 219, 117, 250, 103, 184, 1, 103, 108, 15, 14, 174, 160, 192, 146,
            135, 10, 55, 125, 68, 181, 88, 127, 183, 34, 4, 213, 19, 146, 81, 56, 248, 213, 241,
            168, 205, 253, 29, 10, 123, 198, 177, 157, 247, 209, 113, 167, 231, 42, 214, 15, 12,
            200, 192, 230, 125, 227, 74, 0, 112, 114, 117, 100, 112, 115, 58, 47, 80, 73, 68, 61,
            50, 59, 115, 105, 100, 61, 49, 59, 115, 116, 114, 101, 97, 109, 61, 49, 48, 59, 116,
            121, 112, 101, 61, 50, 59, 97, 100, 100, 114, 101, 115, 115, 61, 57, 49, 46, 57, 56,
            46, 49, 50, 56, 46, 56, 54, 59, 112, 111, 114, 116, 61, 54, 48, 48, 49, 59, 67, 73, 68,
            61, 49, 0, 0, 0, 0, 0, 1, 0, 0, 162, 243, 240, 168, 31, 0, 0, 0, 51, 0, 98, 114, 97,
            110, 99, 104, 58, 111, 114, 105, 103, 105, 110, 47, 112, 114, 111, 106, 101, 99, 116,
            47, 119, 117, 112, 45, 97, 103, 109, 106, 32, 98, 117, 105, 108, 100, 58, 51, 95, 56,
            95, 49, 53, 95, 50, 48, 48, 52, 95, 48, 0,
        ];
        let rmc_packet = RMCResponse::new(&mut Cursor::new(&packet)).unwrap();
        println!("{:?}", rmc_packet);

        let RMCResponseResult::Success {
            call_id,
            method_id,
            data,
        } = rmc_packet.response_result
        else {
            panic!();
        };

        println!("{}", hex::encode(&data));

        let mut data =
            <(QResult, PID, Vec<u8>, ConnectionData, String) as RmcSerialize>::deserialize(
                &mut Cursor::new(&data[..]),
            )
            .unwrap();

        println!("{:?}", data);

        let key = derive_key(1110, "AAAAAAAAAAAAAAAA".as_bytes());

        let mut rc4 = Rc4::new((&key).into());

        rc4.apply_keystream(&mut data.2);
        println!("raw tick: {:?}", data.2);

        let tick: &kerberos::Ticket =
            bytemuck::from_bytes(&data.2[..size_of::<kerberos::Ticket>()]);

        let remainder = &data.2[size_of::<kerberos::Ticket>()..];

        println!("tick: {:?}", tick);
        let data = <Vec<u8> as RmcSerialize>::deserialize(&mut Cursor::new(remainder)).unwrap();
        println!("inner ticket raw: {:?}", data);

        println!("{:?}", data);
    }
}
