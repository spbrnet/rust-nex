use std::{env, hash::{DefaultHasher, Hasher}, net::SocketAddrV4, sync::LazyLock};
use std::str::FromStr;
use cfg_if::cfg_if;
use rnex_auth_protos::{
    LocalAuthProtocol,
    auth::{Auth, AuthenticationInfo, ConnectionData, ConnectionDataOld},
};
use rnex_prudp::kerberos::{Ticket, TicketInternalData};
use rnex_rmc::{
    any::Any,
    qresult::QResult,
    rand,
    response::ErrorCode,
    rmc_struct,
    util::{
        PID,
        account::Account,
        date_time::DateTime,
        nnas::{NnasError, validate_nex_token},
    },
};
use rnex_server::PassthroughInitModule;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::{AuthManager, is_maintenance};

#[derive(Debug)]
#[rmc_struct(AuthProtocol)]
pub struct AuthHandler {
    pub(crate) am: PassthroughInitModule<AuthManager>,
    pub(crate) authenticated_account: RwLock<Option<Account>>,
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

fn try_get_env<T: FromStr>(name: &'static str) -> Result<T, ErrorCode> {
    env::var(name)
        .ok()
        .and_then(|val| val.parse::<T>().ok())
        .ok_or(ErrorCode::Core_Unknown)
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
    async fn validate_account(
        &self,
        name: &str,
        extra_data: &Any,
    ) -> Result<Account, ErrorCode> {
        let authentication = extra_data
            .try_get_as::<AuthenticationInfo>()
            .map_err(|_| ErrorCode::Authentication_InvalidParam)?;
        let account = validate_nex_token(&authentication.auth_token)
            .await
            .map_err(|error| {
                match error {
                    NnasError::InvalidToken => warn!("NNAS rejected a NEX token"),
                    _ => warn!("NNAS token validation failed: {error}"),
                }
                ErrorCode::RendezVous_NotAuthenticated
            })?;
        let pid = account.rnex_pid();
        if name.parse::<PID>().ok() != Some(pid) {
            warn!("NEX login name did not match the validated token PID");
            return Err(ErrorCode::Authentication_PrincipalIdUnmatched);
        }

        Ok(Account::new(pid, &account.username, &account.nex_password))
    }

    async fn remember_account(&self, account: Account) {
        *self.authenticated_account.write().await = Some(account);
    }

    async fn remembered_login_data(&self, pid: PID) -> Option<(PID, [u8; 16])> {
        if pid == GUEST_ACCOUNT.pid {
            return Some(GUEST_ACCOUNT.get_login_data());
        }
        self.authenticated_account
            .read()
            .await
            .as_ref()
            .filter(|account| account.pid == pid)
            .map(Account::get_login_data)
    }

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
        let Ok(pid) = name.parse::<PID>() else {
            warn!("unable to connect to parse pid: {}", name);
            return Err(ErrorCode::Core_InvalidArgument);
        };

        warn!("login without a NEX bearer token is not supported for PID {pid}");
        Err(ErrorCode::RendezVous_NotAuthenticated)
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
        if is_maintenance() {
            return Err(ErrorCode::RendezVous_GameServerMaintenance);
        }

        let (pid, ticket) = self.generate_ticket_from_name(&name).await?;

        let result = QResult::success(ErrorCode::Core_Unknown);

        let mut hasher = DefaultHasher::new();

        hasher.write(name.as_bytes());

        // let Ok(addr) = self.am.control_server.get_url(hasher.finish()).await else {
        //     warn!("no secure proxies");
        //     return Err(ErrorCode::Core_Exception);
        // };

        let addr = match try_get_env("FORWARD_DESTINATION") {
            Ok(val) => val,
            Err(e) => return Err(e),
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
                extra_data: Any,
            ) -> Result<(QResult, PID, Vec<u8>, ConnectionData, String, String), ErrorCode> {
                if is_maintenance() {
                    return Err(ErrorCode::RendezVous_GameServerMaintenance);
                }

                let account = self.validate_account(&name, &extra_data).await?;
                let pid = account.pid;
                self.remember_account(account).await;
                let (pid, key, ticket) = self.generate_ticket_from_name_string_user_key(&pid.to_string())?;

                let result = QResult::success(ErrorCode::Core_Unknown);

                let mut hasher = DefaultHasher::new();

                hasher.write(name.as_bytes());

                // let Ok(addr) = self.control_server.get_url(hasher.finish()).await else {
                //     warn!("no secure proxies");
                //     return Err(ErrorCode::Core_Exception);
                // };

                let addr = match try_get_env("FORWARD_DESTINATION") {
                    Ok(val) => val,
                    Err(e) => return Err(e),
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
                let Some((pid, _)) = self.remembered_login_data(source_pid).await else {
                    return Err(ErrorCode::Core_Exception);
                };

                let desgination_login_data = if destination_pid == self.am.destination_server_acct.pid {
                    self.am.destination_server_acct.get_login_data()
                } else {
                    return Err(ErrorCode::RendezVous_InvalidOperation);
                };

                let result = QResult::success(ErrorCode::Core_Unknown);

                let ticket = generate_ticket_with_string_user_key(pid, desgination_login_data);

                Ok((result, ticket.1.into(), ticket.0))
            }
        } else {
            async fn login_ex(
                &self,
                name: String,
                extra_data: Any,
            ) -> Result<(QResult, PID, Vec<u8>, ConnectionData, String), ErrorCode> {
                if is_maintenance() {
                    return Err(ErrorCode::RendezVous_GameServerMaintenance);
                }

                let account = self.validate_account(&name, &extra_data).await?;
                let source_login_data = account.get_login_data();
                let pid = account.pid;
                let ticket = generate_ticket(
                    source_login_data,
                    self.am.destination_server_acct.get_login_data(),
                );
                self.remember_account(account).await;

                let result = QResult::success(ErrorCode::Core_Unknown);

                let mut hasher = DefaultHasher::new();

                hasher.write(name.as_bytes());

                // let Ok(addr) = self.am.control_server.get_url(hasher.finish()).await else {
                //     warn!("no secure proxies");
                //     return Err(ErrorCode::Core_Exception);
                // };

                let addr = match try_get_env("FORWARD_DESTINATION") {
                    Ok(val) => val,
                    Err(e) => return Err(e),
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
                let Some((pid, passwd)) = self.remembered_login_data(source_pid).await else {
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
#[cfg(test)]
mod test {
    use std::io::Cursor;

    use rnex_auth_protos::auth::ConnectionData;
    use rnex_rmc::{
        qresult::QResult,
        serialization::RmcSerialize,
        util::{PID, date_time::DateTime},
    };

    type A = (QResult, PID, Vec<u8>, ConnectionData, String);

    #[test]
    fn test() {
        let data: Vec<u8> = vec![
            117, 78, 111, 185, 170, 86, 1, 87, 12, 184, 207, 248, 138, 244, 200, 253, 115, 80, 239,
            214, 101, 196, 158, 106, 17, 107, 196, 210, 174, 2, 57, 126, 192, 37, 185, 250, 1, 237,
            21, 26, 95, 138, 247, 179, 204, 145, 61, 62, 68, 192, 16, 57, 73, 59, 123, 29, 219,
            181, 235, 252, 19, 241, 47, 54, 215, 231, 0, 42, 20, 15, 139, 27, 135, 88, 25, 193,
            172, 242, 13, 244, 128, 118, 37, 244, 102, 138, 8, 40, 182, 242, 146, 92, 104, 53, 4,
            52, 212, 47, 145, 120, 8, 78, 127, 150, 29, 210, 68, 203, 36, 241, 96, 189, 18, 153,
            109, 121,
        ];
        let data: A = (
            QResult(65537),
            1132,
            data,
            ConnectionData {
                station_url:
                    "prudps:/PID=2;sid=1;stream=10;type=2;address=45.85.147.85;port=17001;CID=1"
                        .into(),
                special_protocols: [].into(),
                special_station_url: "".into(),
                date_time: DateTime(135993837066),
            },
            "branch:origin/project/wup-agmj build:3_8_15_2004_0".to_owned(),
        );

        let test = data.to_data().unwrap();
        A::deserialize(&mut Cursor::new(&test[..])).unwrap();
    }
}
