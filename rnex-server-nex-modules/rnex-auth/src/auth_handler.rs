use std::{
    hash::{DefaultHasher, Hasher},
    net::SocketAddrV4,
    sync::{LazyLock},
};

use cfg_if::cfg_if;
use nex_account::{
    grpc,
    grpc_client,
};
use rnex_auth_protos::{
    LocalAuthProtocol,
    auth::{Auth, ConnectionData, ConnectionDataOld},
};
use rnex_prudp::kerberos::{Ticket, TicketInternalData};
use rnex_reggie_protos::reggie::{RemoteEdgeNodeManagement};
use rnex_rmc::{
    any::Any,
    qresult::QResult,
    rand,
    response::ErrorCode,
    rmc_struct,
    util::{PID, account::Account, date_time::DateTime},
};
use rnex_server::PassthroughInitModule;
use tracing::{info, warn};

use crate::AuthManager;

#[derive(Debug)]
#[rmc_struct(AuthProtocol)]
pub struct AuthHandler {
    pub(crate) am: PassthroughInitModule<AuthManager>,
}

pub fn generate_ticket(
    source_act_login_data: (PID, [u8; 16]),
    dest_act_login_data: (PID, [u8; 16]),
) -> Box<[u8]> {
    let source_key = source_act_login_data.1;
    let dest_key = dest_act_login_data.1;

    let internal_data = TicketInternalData::new(source_act_login_data.0);

    let encrypted_inner = internal_data.encrypt(dest_key);
    Ticket {
        pid: dest_act_login_data.0,
        session_key: internal_data.session_key,
    }
    .encrypt(source_key, &encrypted_inner)
}
pub fn generate_ticket_with_string_user_key(
    source_act: PID,
    dest_act_login_data: (PID, [u8; 16]),
) -> (String, Box<[u8]>) {
    let source_key: [u8; 8] = rand::random();
    let key_string = hex::encode(source_key);
    let key_data: [u8; 16] = key_string.as_bytes().try_into().unwrap();
    let dest_key = dest_act_login_data.1;

    let internal_data = TicketInternalData::new(source_act);

    let encrypted_inner = internal_data.encrypt(dest_key);
    let encrypted_session_ticket = Ticket {
        pid: dest_act_login_data.0,
        session_key: internal_data.session_key,
    }
    .encrypt(key_data, &encrypted_inner);

    (key_string, encrypted_session_ticket)
}

async fn get_login_data_by_pid(pid: PID) -> Option<(PID, [u8; 16])> {
    if pid == GUEST_ACCOUNT.pid {
        let source_login_data = GUEST_ACCOUNT.get_login_data();

        return Some((source_login_data.0, source_login_data.1));
    }

    let Ok(mut client) = nex_account::grpc_client().await else {
        return None;
    };

    let Ok(passwd) = client.get_nex_key_by_pid(grpc::Pid { pid }).await else {
        return None;
    };

    let passwd = passwd.into_inner().key.try_into().ok()?;

    Some((pid, passwd))
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
                info!("guest account login");
                let source_login_data = GUEST_ACCOUNT.get_login_data();
                let destination_login_data = self.am.destination_server_acct.get_login_data();

                return Ok((
                    source_login_data.0,
                    generate_ticket(source_login_data, destination_login_data),
                ));
            }
        }

        info!("parsing pid");
        let Ok(pid) = name.parse() else {
            warn!("unable to connect to parse pid: {}", name);
            return Err(ErrorCode::Core_InvalidArgument);
        };

        info!("creating account grpc client");
        let Ok(mut client) = grpc_client().await else {
            warn!("unable to connect to grpc");
            return Err(ErrorCode::Core_Exception);
        };

        info!("grabbing nex key");
        let Ok(passwd) = client.get_nex_key_by_pid(grpc::Pid { pid }).await else {
            warn!("unable to get nex password for pid: {}:", pid);
            return Err(ErrorCode::Core_Exception);
        };

        let passwd = passwd
            .into_inner()
            .key
            .try_into()
            .map_err(|_| ErrorCode::RendezVous_InvalidPassword)?;
        info!("source login data");
        let source_login_data = (pid, passwd);
        println!("{}, {:?}", pid, passwd);
        let destination_login_data = self.am.destination_server_acct.get_login_data();

        info!("we are a-ok here");
        Ok((
            pid,
            generate_ticket(source_login_data, destination_login_data),
        ))
    }

    pub fn generate_ticket_from_name_string_user_key(
        &self,
        name: &str,
    ) -> Result<(PID, String, Box<[u8]>), ErrorCode> {
        {
            if name == GUEST_ACCOUNT.username {
                let source_login_data = GUEST_ACCOUNT.get_login_data();
                let destination_login_data = self.am.destination_server_acct.get_login_data();
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
        let destination_login_data = self.am.destination_server_acct.get_login_data();

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

        let result = QResult::success(ErrorCode::Core_Unknown);

        let mut hasher = DefaultHasher::new();

        hasher.write(name.as_bytes());

        let Ok(addr) = self.am.control_server.get_url(hasher.finish()).await else {
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
            self.am.build_name.to_string(),
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

                let result = QResult::success(ErrorCode::Core_Unknown);

                let mut hasher = DefaultHasher::new();

                hasher.write(name.as_bytes());

                let Ok(addr) = self.am.control_server.get_url(hasher.finish()).await else {
                    warn!("no secure proxies");
                    return Err(ErrorCode::Core_Exception);
                };

                let connection_data = ConnectionData {
                    station_url: station_url_from_sock_addr(addr),
                    special_station_url: "".to_string(),
                    //date_time: KerberosDateTime::new(1,1,1,1,1,1),
                    date_time: DateTime::now(),
                    special_protocols: Vec::new(),
                };

                let ret = (
                    result,
                    pid,
                    ticket.into(),
                    connection_data,
                    self.am.build_name.to_string(),
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

                let desgination_login_data = if destination_pid == self.am.destination_server_acct.pid {
                    self.am.destination_server_acct.get_login_data()
                } else {
                    return Err(ErrorCode::RendezVous_InvalidOperation);
                };

                let result = QResult::success(ErrorCode::Core_Unknown);

                let ticket = generate_ticket((pid, passwd), desgination_login_data);

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
