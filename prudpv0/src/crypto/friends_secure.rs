use hmac::Mac;
use rc4::Rc4;
use rnex_core::prudp::encryption::EncryptionPair;
use typenum::U32;

use crate::crypto::{
    Crypto, CryptoInstance,
    common_crypto::common_checksum,
    friends_common::{ACCESS_KEY, HmacMd5},
};

pub struct SecureInstance {
    pair: EncryptionPair<Rc4<U32>>,
}

impl CryptoInstance for SecureInstance {
    fn decrypt_incoming(&mut self, data: &mut [u8]) {
        todo!()
    }
    fn encrypt_outgoing(&mut self, data: &mut [u8]) {
        todo!()
    }
    fn get_user_id(&self) -> u32 {
        todo!()
    }
    fn generate_signature(&self, data: &[u8]) -> [u8; 4] {
        let mut hmac = <HmacMd5 as Mac>::new_from_slice(ACCESS_KEY.as_bytes())
            .expect("unable to create hmac md5");
        hmac.update(data);
        hmac.finalize().into_bytes()[0..4].try_into().unwrap()
    }
}

pub struct Secure();

impl Crypto for Secure {
    type Instance = SecureInstance;
    fn new() -> Self {
        Self()
    }
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        common_checksum(ACCESS_KEY, data)
    }
    fn instantiate(&self, data: &[u8]) -> Self::Instance {
        todo!()
    }
}
