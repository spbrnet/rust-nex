use once_cell::sync::Lazy;
use rnex_core::nex::account::Account;
use rnex_core::rmc::protocols::{RmcCallable, RmcConnection, new_rmc_gateway_connection};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use std::env;
use std::io::Cursor;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::TcpListener;
use std::sync::LazyLock;
use std::sync::OnceLock;
use sqlx::postgres::PgPool;

use log::error;
use std::error::Error;
use std::string::ToString;
use crate::reggie::UnitPacketRead;

const IP_REQ_SERVICE_URL: &str = "https://ipinfo.io/ip";

pub static RNEX_DATASTORE_DATABASE_URL: LazyLock<String> = LazyLock::new(|| {
    std::env::var("RNEX_DATASTORE_DATABASE_URL")
        .expect("RNEX_DATASTORE_DATABASE_URL must be set")
});

pub static DB_POOL: OnceLock<PgPool> = OnceLock::new();

pub fn get_db() -> &'static PgPool {
    DB_POOL.get().expect("db_pool not initialized")
}
pub static RNEX_DATASTORE_S3_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
    std::env::var("RNEX_DATASTORE_S3_ENDPOINT")
        .expect("RNEX_DATASTORE_S3_ENDPOINT must be set")
});
pub static RNEX_DATASTORE_S3_BUCKET: LazyLock<String> = LazyLock::new(|| {
    std::env::var("RNEX_DATASTORE_S3_BUCKET")
        .expect("RNEX_DATASTORE_S3_BUCKET must be set")
});

pub fn try_get_ip() -> Result<Ipv4Addr, Box<dyn Error>> {
    let mut req = ureq::get(IP_REQ_SERVICE_URL).call()?;

    Ok(req.body_mut().read_to_string()?.parse()?)
}

pub static OWN_IP_PRIVATE: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP")
        .ok()
        .map(|s| s.parse().expect("invalid ip address"))
        .unwrap_or(Ipv4Addr::UNSPECIFIED)
});

pub static OWN_IP_PUBLIC: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP_PUBLIC")
        .ok()
        .map(|s| s.parse().expect("invalid ip address"))
        .unwrap_or_else(|| try_get_ip().unwrap())
});

pub static SERVER_PORT: Lazy<u16> = Lazy::new(|| {
    env::var("SERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10000)
});

pub static KERBEROS_SERVER_PASSWORD: Lazy<String> = Lazy::new(|| {
    env::var("AUTH_SERVER_PASSWORD")
        .ok()
        .unwrap_or("password".to_owned())
});

pub static AUTH_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(1, "Quazal Authentication", &KERBEROS_SERVER_PASSWORD));
pub static SECURE_SERVER_ACCOUNT: Lazy<Account> =
    Lazy::new(|| Account::new(2, "Quazal Rendez-Vous", &KERBEROS_SERVER_PASSWORD));

pub async fn new_simple_backend<T: RmcCallable + Sync + Send + 'static, F>(mut creation_function: F)
where
    F: FnMut(ConnectionInitData, RmcConnection) -> Arc<T>,
{
    let listen = TcpListener::bind(SocketAddrV4::new(*OWN_IP_PRIVATE, *SERVER_PORT))
        .await
        .unwrap();
    while let Ok((mut stream, _addr)) = listen.accept().await {
        let buffer = match stream.read_buffer().await {
            Ok(v) => v,
            Err(e) => {
                error!(
                    "an error ocurred whilest reading connection data buffer: {:?}",
                    e
                );
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let user_connection_data = match user_connection_data {
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilest reading connection data: {:?}", e);
                continue;
            }
        };
        let fun_ref = &mut creation_function;
        new_rmc_gateway_connection(stream.into(), move |r| fun_ref(user_connection_data, r));
    }
}
