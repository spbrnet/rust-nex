use rc4::StreamCipher;

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

pub const DEFAULT_KEY: &[u8; 5] = b"CD&ML";
