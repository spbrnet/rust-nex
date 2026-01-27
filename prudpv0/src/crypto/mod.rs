use cfg_if::cfg_if;
use rnex_core::prudp::types_flags::TypesFlags;

mod common_crypto;

pub trait CryptoInstance: Send + 'static {
    fn decrypt_incoming(&mut self, data: &mut [u8]);
    fn encrypt_outgoing(&mut self, data: &mut [u8]);
    fn generate_signature(&self, types_flags: TypesFlags, data: &[u8]) -> [u8; 4];
    fn get_user_id(&self) -> u32;
}

pub trait Crypto: Send + Sync + 'static {
    type Instance: CryptoInstance;
    fn new() -> Self;
    fn calculate_checksum(&self, data: &[u8]) -> u8;
    fn instantiate(
        &self,
        data: &[u8],
        self_signat: [u8; 4],
        remote_signat: [u8; 4],
    ) -> Self::Instance;
}

cfg_if! {
    if #[cfg(feature = "friends")]{
        pub mod friends_common;
        pub mod friends_insecure;
        pub use friends_insecure::*;
        pub mod friends_secure;
        pub use friends_secure::*;
    } else {
        pub mod secure;
        pub use secure::*;
        pub mod insecure;
        pub use insecure::*;
    }
}
