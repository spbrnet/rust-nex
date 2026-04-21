use cfg_if::cfg_if;

pub mod account;
pub mod auth_handler;
pub mod common;
pub mod friends_handler;
pub mod matchmake;
pub mod remote_console;
pub mod user;
pub mod datastore;
cfg_if! {
    if #[cfg(feature = "datastore")] {
        pub mod s3presigner;
    }
}