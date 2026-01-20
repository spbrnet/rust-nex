use std::rc::Rc;

use hmac::Mac;
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_core::prudp::encryption::{DEFAULT_KEY, EncryptionPair};
use typenum::U5;

use crate::crypto::{
    Crypto, CryptoInstance,
    common_crypto::common_checksum,
    friends_common::{ACCESS_KEY, HmacMd5},
};

pub struct InsecureInstance {
    pair: EncryptionPair<Rc4<U5>>,
}

impl CryptoInstance for InsecureInstance {
    fn decrypt_incoming(&mut self, data: &mut [u8]) {
        self.pair.recv.apply_keystream(data);
    }
    fn encrypt_outgoing(&mut self, data: &mut [u8]) {
        self.pair.send.apply_keystream(data);
    }
    fn get_user_id(&self) -> u32 {
        0
    }
    fn generate_signature(&self, data: &[u8]) -> [u8; 4] {
        let mut hmac = <HmacMd5 as Mac>::new_from_slice(ACCESS_KEY.as_bytes())
            .expect("unable to create hmac md5");
        hmac.update(data);
        hmac.finalize().into_bytes()[0..4].try_into().unwrap()
    }
}

pub struct Insecure();

impl Crypto for Insecure {
    type Instance = InsecureInstance;
    fn new() -> Self {
        Self()
    }
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        common_checksum(ACCESS_KEY, data)
    }

    fn instantiate(&self, packet_data: &[u8]) -> Self::Instance {
        InsecureInstance {
            pair: EncryptionPair::init_both(|| Rc4::new(&DEFAULT_KEY)),
        }
    }
}
