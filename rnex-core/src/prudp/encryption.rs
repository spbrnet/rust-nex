use std::sync::LazyLock;

use rc4::{Key, StreamCipher};
use typenum::U5;

pub struct EncryptionPair<T: StreamCipher + Send> {
    pub send: T,
    pub recv: T,
}

impl<T: StreamCipher + Send> EncryptionPair<T> {
    pub fn init_both<F: Fn() -> T>(func: F) -> Self {
        Self {
            recv: func(),
            send: func(),
        }
    }
}

pub static DEFAULT_KEY: LazyLock<Key<U5>> = LazyLock::new(|| Key::from(*b"CD&ML"));
