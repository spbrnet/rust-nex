use crate::prudp::packet::PRUDPV1Packet;
use crate::prudp::socket::{CryptoHandler, CryptoHandlerConnectionInstance};
use hmac::digest::consts::U32;
use rc4::cipher::StreamCipherCoreWrapper;
use rc4::{KeyInit, Rc4, Rc4Core, StreamCipher};
use rnex_core::PID;
use rnex_core::nex::account::Account;
use rnex_core::prudp::encryption::EncryptionPair;
use rnex_core::prudp::ticket::read_secure_connection_data;
use rnex_core::rmc::structures::RmcSerialize;
use typenum::U5;

type Rc4U32 = StreamCipherCoreWrapper<Rc4Core<U32>>;

pub fn generate_secure_encryption_pairs(
    mut session_key: [u8; 32],
    count: u8,
) -> Vec<EncryptionPair<Rc4<U32>>> {
    let mut vec = Vec::with_capacity(count as usize);

    vec.push(EncryptionPair {
        send: Rc4U32::new_from_slice(&session_key).expect("unable to create rc4"),
        recv: Rc4U32::new_from_slice(&session_key).expect("unable to create rc4"),
    });

    for _ in 1..=count {
        let modifier = session_key.len() + 1;

        let key_length = session_key.len();

        for (position, val) in (&mut session_key[0..key_length / 2]).iter_mut().enumerate() {
            *val = val.wrapping_add((modifier - position) as u8);
        }

        vec.push(EncryptionPair {
            send: Rc4U32::new_from_slice(&session_key).expect("unable to create rc4"),
            recv: Rc4U32::new_from_slice(&session_key).expect("unable to create rc4"),
        });
    }

    vec
}

pub struct Secure(pub &'static str, pub Account);

pub struct SecureInstance {
    access_key: &'static str,
    session_key: [u8; 32],
    streams: Vec<EncryptionPair<Rc4<U32>>>,
    self_signature: [u8; 16],
    #[allow(dead_code)]
    remote_signature: [u8; 16],
    pid: PID,
}

impl CryptoHandler for Secure {
    type CryptoConnectionInstance = SecureInstance;

    fn instantiate(
        &self,
        remote_signature: [u8; 16],
        self_signature: [u8; 16],
        payload: &[u8],
        substream_count: u8,
    ) -> Option<(Vec<u8>, Self::CryptoConnectionInstance)> {
        let (session_key, pid, check_value) = read_secure_connection_data(payload, &self.1)?;

        let check_value_response = check_value + 1;

        let data = bytemuck::bytes_of(&check_value_response);

        let mut response = Vec::new();

        data.serialize(&mut response).ok()?;

        let encryption_pairs = generate_secure_encryption_pairs(session_key, substream_count);

        Some((
            response,
            SecureInstance {
                pid,
                streams: encryption_pairs,
                session_key,
                access_key: self.0,
                remote_signature,
                self_signature,
            },
        ))
    }

    fn sign_pre_handshake(&self, packet: &mut PRUDPV1Packet) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.0, None, None);
    }
}

impl CryptoHandlerConnectionInstance for SecureInstance {
    type Encryption = Rc4<U5>;

    fn decrypt_incoming(&mut self, substream: u8, data: &mut [u8]) {
        if let Some(crypt_pair) = self.streams.get_mut(substream as usize) {
            crypt_pair.recv.apply_keystream(data);
        }
    }

    fn encrypt_outgoing(&mut self, substream: u8, data: &mut [u8]) {
        if let Some(crypt_pair) = self.streams.get_mut(substream as usize) {
            crypt_pair.send.apply_keystream(data);
        }
    }

    fn get_user_id(&self) -> i32 {
        self.pid
    }

    fn sign_connect(&self, packet: &mut PRUDPV1Packet) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.access_key, None, Some(self.self_signature));
    }

    fn sign_packet(&self, packet: &mut PRUDPV1Packet) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(
            self.access_key,
            Some(self.session_key),
            Some(self.self_signature),
        );
    }

    fn verify_packet(&self, _packet: &PRUDPV1Packet) -> bool {
        true
    }
}
