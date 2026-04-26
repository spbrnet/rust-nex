use cfg_if::cfg_if;

pub mod account;
pub mod auth_handler;
pub mod common;

cfg_if! {
    if #[cfg(feature = "friends")]{
        pub mod friends_handler;
    } else {
        pub mod matchmake;
        pub mod remote_console;
        pub mod user;
    }
}
