use crate::reggie::UnitPacketRead;
use cfg_if::cfg_if;
use log::error;
use once_cell::sync::Lazy;
use rnex_core::nex::account::Account;
use rnex_core::rmc::protocols::{RmcCallable, RmcConnection, new_rmc_gateway_connection};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rnex_proxy_common::ConnectionInitData;
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::io::{Cursor, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::sync::Arc;
use tokio::net::TcpListener;

const IP_REQ_SERVICE_URLS: &[(&str, &str, &str)] = &[
    ("ipinfo.io:80", "ipinfo.io", "/ip"),
    ("api.ipify.org:80", "api.ipify.org", "/"),
    // preresolved
    ("34.117.59.81:80", "ipinfo.io", "/ip"),
    ("104.26.13.205:80", "api.ipify.org", "/"),
    ("172.67.74.152:80", "api.ipify.org", "/"),
    ("104.26.12.205:80", "api.ipify.org", "/"),
];

cfg_if! {
    if #[cfg(feature = "database-support")] {
        use std::sync::{LazyLock, OnceLock};
        use sqlx::postgres::PgPool;
        pub static RNEX_DATABASE_URL: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATABASE_URL")
                .expect("RNEX_DATABASE_URL must be set")
        });

        pub static DB_POOL: OnceLock<PgPool> = OnceLock::new();

        pub fn get_db() -> &'static PgPool {
            DB_POOL.get().expect("db_pool not initialized")
        }
    }
}
cfg_if! {
    if #[cfg(feature = "datastore")]{
        pub static RNEX_DATASTORE_S3_ENDPOINT: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATASTORE_S3_ENDPOINT")
                .expect("RNEX_DATASTORE_S3_ENDPOINT must be set")
        });
        pub static RNEX_DATASTORE_S3_BUCKET: LazyLock<String> = LazyLock::new(|| {
            std::env::var("RNEX_DATASTORE_S3_BUCKET")
                .expect("RNEX_DATASTORE_S3_BUCKET must be set")
        });
    }
}

pub fn try_to_log<R, E: Display>(fun: impl FnOnce() -> Result<R, E>) -> Option<R> {
    match fun() {
        Ok(v) => Some(v),
        Err(e) => {
            println!("{}", e);
            None
        }
    }
}

pub fn try_get_ip() -> Option<Ipv4Addr> {
    for url in IP_REQ_SERVICE_URLS {
        println!("trying to get ip via: {:?}", url);
        if let Some(v) = try_to_log::<_, Box<dyn Error>>(|| {
            let mut stream = TcpStream::connect(url.0)?;
            stream.write_all(
                format!(
                    r#"GET {} HTTP/1.0
Host: {}
User-Agent: RNEX
Accept: */*

"#,
                    url.2, url.1
                )
                .as_str()
                .as_bytes(),
            )?;
            let mut data = vec![];
            stream.read_to_end(&mut data)?;
            let string = String::from_utf8(data)?;
            let (_, ip) = string
                .split_once("\r\n\r\n")
                .ok_or("unable to get ip from response")?;
            Ok(ip.parse()?)
        }) {
            return Some(v);
        }
    }
    None
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
                    "an error ocurred whilst reading connection data buffer: {:?}",
                    e
                );
                continue;
            }
        };

        let user_connection_data = ConnectionInitData::deserialize(&mut Cursor::new(buffer));

        let user_connection_data = match user_connection_data {
            Ok(v) => v,
            Err(e) => {
                error!("an error ocurred whilst reading connection data: {:?}", e);
                continue;
            }
        };
        let fun_ref = &mut creation_function;
        new_rmc_gateway_connection(stream.into(), move |r| fun_ref(user_connection_data, r));
    }
}

#[cfg(test)]
mod test {
    use std::net::ToSocketAddrs;

    use crate::executables::common::{IP_REQ_SERVICE_URLS, try_get_ip};

    #[test]
    fn get_ip() {
        println!("{}", try_get_ip().unwrap());
    }
}
