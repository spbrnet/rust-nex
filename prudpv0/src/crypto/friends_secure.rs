use hmac::Mac;
use md5::{Digest, Md5};
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_prudp::{
    encryption::EncryptionPair,
    ticket::read_secure_connection_data,
    types_flags::{TypesFlags, types::DATA},
};
use rnex_rmc::serialization::RmcSerialize;
use rnex_util::{PID, account::Account};
use crate::crypto::{
    Crypto, CryptoInstance,
    common_crypto::common_checksum,
    friends_common::{ACCESS_KEY, HmacMd5},
};

pub struct SecureInstance {
    pair: EncryptionPair<Rc4>,
    uid: PID,
    self_signat: [u8; 4],
    #[allow(dead_code)]
    remote_signat: [u8; 4],
}

impl CryptoInstance for SecureInstance {
    fn decrypt_incoming(&mut self, data: &mut [u8]) {
        self.pair.recv.apply_keystream(data);
    }
    fn encrypt_outgoing(&mut self, data: &mut [u8]) {
        self.pair.send.apply_keystream(data);
    }
    fn get_user_id(&self) -> PID {
        self.uid
    }
    fn generate_signature(&self, types_flags: TypesFlags, data: &[u8]) -> [u8; 4] {
        if types_flags.get_types() == DATA {
            if data.len() == 0 {
                [0x78, 0x56, 0x34, 0x12]
            } else {
                let mut hash = Md5::new();
                hash.update(ACCESS_KEY.as_bytes());
                let mut hmac = HmacMd5::new_from_slice(&hash.finalize().as_slice())
                    .expect("unable to create hmac md5");
                hmac.update(data);
                hmac.finalize().into_bytes()[0..4].try_into().unwrap()
            }
        } else {
            self.self_signat
        }
    }
}

pub struct Secure(Account);

impl Crypto for Secure {
    type Instance = SecureInstance;
    async fn new() -> Self {
        Self(
            Account::from_nexact(2, "Quazal Rendez-Vous")
                .await
                .expect("unable to get account info"),
        )
    }
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        common_checksum(ACCESS_KEY, data)
    }
    fn instantiate(
        &self,
        data: &[u8],
        self_signat: [u8; 4],
        remote_signat: [u8; 4],
    ) -> Option<(Self::Instance, Vec<u8>)> {
        let (session_key, pid, check_value) = read_secure_connection_data(data, &self.0)?;

        let check_value_response = check_value + 1;

        let data = bytemuck::bytes_of(&check_value_response);

        let mut response = Vec::new();

        data.serialize(&mut response).ok()?;

        Some((
            SecureInstance {
                pair: EncryptionPair::init_both(|| {
                    Rc4::new_from_slice(&session_key).expect("unable to initialize rc4 stream")
                }),
                self_signat,
                remote_signat,
                uid: pid,
            },
            response,
        ))
    }
}
