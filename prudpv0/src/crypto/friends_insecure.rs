use std::{io::Write, rc::Rc};

use hmac::Mac;
use md5::{Digest, Md5};
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_core::prudp::{
    encryption::{DEFAULT_KEY, EncryptionPair},
    types_flags::{TypesFlags, types::DATA},
};
use typenum::U5;

use crate::crypto::{
    Crypto, CryptoInstance,
    common_crypto::common_checksum,
    friends_common::{ACCESS_KEY, HmacMd5},
};

pub struct InsecureInstance {
    pair: EncryptionPair<Rc4<U5>>,
    self_signat: [u8; 4],
    remote_signat: [u8; 4],
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
    fn generate_signature(&self, types_flags: TypesFlags, data: &[u8]) -> [u8; 4] {
        if types_flags.get_types() == DATA {
            if data.len() == 0 {
                [0x78, 0x56, 0x34, 0x12]
            } else {
                let mut hash = Md5::new();
                hash.write(ACCESS_KEY.as_bytes()).unwrap();
                let mut hmac = <HmacMd5 as Mac>::new_from_slice(&hash.finalize().as_slice())
                    .expect("unable to create hmac md5");
                hmac.update(data);
                hmac.finalize().into_bytes()[0..4].try_into().unwrap()
            }
        } else {
            self.self_signat
        }
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

    fn instantiate(
        &self,
        packet_data: &[u8],
        self_signat: [u8; 4],
        remote_signat: [u8; 4],
    ) -> Option<(Self::Instance, Vec<u8>)> {
        Some((
            InsecureInstance {
                pair: EncryptionPair::init_both(|| Rc4::new(&DEFAULT_KEY)),
                self_signat,
                remote_signat,
            },
            vec![],
        ))
    }
}
