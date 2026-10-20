use cfg_if::cfg_if;
#[cfg(feature = "nx")]
use rnex_rmc::util::mystiko::{AccountError, get_nex_login_data, validate_nex_token};
#[cfg(not(feature = "nx"))]
use rnex_rmc::util::nnas::{NnasError as AccountError, get_nex_login_data, validate_nex_token};
#[cfg(feature = "nx")]
use rnex_auth_protos::auth::{ValidateAndRequestTicketParam, ValidateAndRequestTicketResult};
use rnex_auth_protos::{
    LocalAuthProtocol,
    auth::{Auth, AuthenticationInfo, ConnectionData, ConnectionDataOld},
};
#[cfg(not(feature = "prudplite"))]
use rnex_prudp::kerberos::{Ticket, TicketInternalData};
#[cfg(feature = "prudplite")]
use rnex_prudp::lite::{derive_key, issue_ticket};
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
    },
};
use rnex_server::PassthroughInitModule;
#[cfg(not(feature = "prudplite"))]
use std::{env, net::SocketAddrV4, str::FromStr};
use std::{
    hash::{DefaultHasher, Hasher},
    sync::LazyLock,
};
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
    #[cfg(feature = "prudplite")]
    {
        return issue_ticket(
            source_act_login_data.0 as u64,
            dest_act_login_data.0 as u64,
            &source_act_login_data.1,
            &dest_act_login_data.1,
        )
        .expect("valid nx ticket")
        .into_boxed_slice();
    }
    #[cfg(not(feature = "prudplite"))]
    {
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
}

#[cfg(not(feature = "prudplite"))]
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
    let dest_key = dest_act_login_data.1;

    #[cfg(feature = "prudplite")]
    {
        let source_key: [u8; 16] = rand::random();
        let key_string = hex::encode(source_key);
        let ticket = issue_ticket(
            source_act as u64,
            dest_act_login_data.0 as u64,
            &source_key,
            &dest_key,
        )
        .expect("valid nx ticket")
        .into_boxed_slice();
        return (key_string, ticket);
    }

    #[cfg(not(feature = "prudplite"))]
    {
        let source_key: [u8; 8] = rand::random();
        let key_string = hex::encode(source_key);
        let key_data: [u8; 16] = key_string.as_bytes().try_into().unwrap();
        let internal_data = TicketInternalData::new(source_act);

        let encrypted_inner = internal_data.encrypt(dest_key);
        let encrypted_session_ticket = Ticket {
            pid: dest_act_login_data.0,
            session_key: internal_data.session_key,
        }
        .encrypt(key_data, &encrypted_inner);

        (key_string, encrypted_session_ticket)
    }
}

#[cfg(not(feature = "prudplite"))]
fn station_url_from_sock_addr(sock_addr: SocketAddrV4) -> String {
    format!(
        "prudps:/PID=2;sid=1;stream=10;type=2;address={};port={};CID=1",
        sock_addr.ip(),
        sock_addr.port()
    )
}

fn secure_station_url() -> Result<String, ErrorCode> {
    #[cfg(feature = "prudplite")]
    {
        Ok("prudps:/address=0.0.0.1;port=1;PID=2;CID=1;type=2;sid=2;stream=10".to_owned())
    }
    #[cfg(not(feature = "prudplite"))]
    {
        let addr = try_get_env::<SocketAddrV4>("FORWARD_DESTINATION")?;
        Ok(station_url_from_sock_addr(addr))
    }
}

fn user_account(pid: PID, username: &str, password: &str) -> Account {
    #[cfg(feature = "prudplite")]
    {
        Account::new_raw_key(pid, username, derive_key(password, pid as u64))
    }
    #[cfg(not(feature = "prudplite"))]
    {
        Account::new(pid, username, password)
    }
}

static GUEST_ACCOUNT: LazyLock<Account> =
    LazyLock::new(|| user_account(100, "guest", "MMQea3n!fsik"));

impl AuthHandler {
    async fn validate_account(&self, extra_data: &Any) -> Result<Account, ErrorCode> {
        let authentication = extra_data
            .try_get_as::<AuthenticationInfo>()
            .map_err(|_| ErrorCode::Authentication_InvalidParam)?;
        let account = validate_nex_token(&authentication.auth_token)
            .await
            .map_err(|error| {
                #[cfg(feature = "nx")]
                match error {
                    AccountError::InvalidToken => warn!("MystikoNX rejected a NEX token"),
                    _ => warn!("MystikoNX token validation failed: {error}"),
                }
                #[cfg(not(feature = "nx"))]
                match error {
                    AccountError::InvalidToken => warn!("NNAS rejected a NEX token"),
                    _ => warn!("NNAS token validation failed: {error}"),
                }
                ErrorCode::RendezVous_NotAuthenticated
            })?;
        let pid = account.rnex_pid();
        Ok(user_account(pid, &account.username, &account.nex_password))
    }

    async fn remember_account(&self, account: Account) {
        *self.authenticated_account.write().await = Some(account);
    }

    async fn remembered_login_data(&self, pid: PID) -> Option<(PID, [u8; 16])> {
        if cfg!(feature = "guest_login") && pid == GUEST_ACCOUNT.pid {
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

        let login_data = get_nex_login_data(pid).await.map_err(|error| {
            warn!("unable to obtain NEX login data for PID {pid}: {error}");
            ErrorCode::RendezVous_NotAuthenticated
        })?;
        let account = user_account(
            login_data.rnex_pid(),
            &login_data.username,
            &login_data.nex_password,
        );
        let source_login_data = account.get_login_data();
        let destination_login_data = self.am.destination_server_acct.get_login_data();

        Ok((
            source_login_data.0,
            generate_ticket(source_login_data, destination_login_data),
        ))
    }

    pub fn generate_ticket_from_name_string_user_key(
        &self,
        name: &str,
    ) -> Result<(PID, String, Box<[u8]>), ErrorCode> {
        {
            if cfg!(feature = "guest_login") && name == GUEST_ACCOUNT.username {
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
    #[cfg(feature = "nx")]
    async fn login_with_context(
        &self,
        request: ValidateAndRequestTicketParam,
    ) -> Result<ValidateAndRequestTicketResult, ErrorCode> {
        if is_maintenance() {
            return Err(ErrorCode::RendezVous_GameServerMaintenance);
        }
        let account = self.validate_account(&request.extra_data).await?;
        let pid = account.pid;
        self.remember_account(account).await;
        let (_, source_key, ticket) =
            self.generate_ticket_from_name_string_user_key(&pid.to_string())?;
        Ok(ValidateAndRequestTicketResult {
            source_pid: pid,
            ticket: ticket.into(),
            service_node_url: secure_station_url()?,
            current_utc_time: DateTime::now(),
            return_message: self.am.build_name.to_string(),
            source_key,
        })
    }
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

        let connection_data = ConnectionDataOld {
            station_url: secure_station_url()?,
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

                let account = self.validate_account(&extra_data).await?;
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

                let connection_data = ConnectionData {
                    station_url: secure_station_url()?,
                    special_station_url: "".to_string(),
                    //date_time: KerberosDateTime::new(1,1,1,1,1,1),
                    date_time: DateTime::now(),
                    special_protocols: Vec::new(),
                };

                info!(
                    pid,
                    station_url = %connection_data.station_url,
                    ticket_bytes = ticket.len(),
                    "NEX LoginEx accepted"
                );

                let ret = (
                    result,
                    pid,
                    ticket.into(),
                    connection_data,
                    self.am.build_name.to_string(),
                    key
                );

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

                let account = self.validate_account(&extra_data).await?;
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

                let connection_data = ConnectionData {
                    station_url: secure_station_url()?,
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
