use cfg_if::cfg_if;

pub mod common;
cfg_if! {
    if #[cfg(feature = "friends")]{
        pub mod friends_backend;
    } else {
        pub mod regular_backend;
    }
}
