use std::env;
use std::net::Ipv4Addr;
use once_cell::sync::Lazy;
use crate::nex::account::Account;

pub static OWN_IP_PRIVATE: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP")
        .ok()
        .map(|s| s.parse().expect("invalid ip address"))
        .unwrap_or(Ipv4Addr::UNSPECIFIED)
});

pub static OWN_IP_PUBLIC: Lazy<Ipv4Addr> = Lazy::new(|| {
    env::var("SERVER_IP_PUBLIC")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("SERVER_IP_PUBLIC not specified")
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



