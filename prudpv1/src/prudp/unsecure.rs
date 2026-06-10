use crate::prudp::packet::PRUDPV1Packet;
use crate::prudp::socket::{CryptoHandler, CryptoHandlerConnectionInstance};
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_core::prudp::encryption::{DEFAULT_KEY, EncryptionPair};
use typenum::U5;

pub struct Unsecure(pub &'static str);

pub struct UnsecureInstance {
    key: &'static str,
    streams: Vec<EncryptionPair<Rc4<U5>>>,
    self_signature: [u8; 16],
    #[allow(dead_code)]
    remote_signature: [u8; 16],
}

// my hand was forced to use lazy so that we can guarantee this code
// only runs once and so that i can put it here as a "constant" (for performance and readability)
// since for some reason rust crypto doesn't have any const time key initialization

impl CryptoHandler for Unsecure {
    type CryptoConnectionInstance = UnsecureInstance;

    fn instantiate(
        &self,
        remote_signature: [u8; 16],
        self_signature: [u8; 16],
        _: &[u8],
        substream_count: u8,
    ) -> Option<(Vec<u8>, Self::CryptoConnectionInstance)> {
        Some((
            Vec::new(),
            UnsecureInstance {
                streams: (0..substream_count)
                    .map(|_| EncryptionPair::init_both(|| Rc4::new(&DEFAULT_KEY)))
                    .collect(),
                key: self.0,
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

impl CryptoHandlerConnectionInstance for UnsecureInstance {
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
        0
    }

    fn sign_connect(&self, packet: &mut PRUDPV1Packet) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.key, None, Some(self.self_signature));
    }

    fn sign_packet(&self, packet: &mut PRUDPV1Packet) {
        packet.set_sizes();
        packet.calculate_and_assign_signature(self.key, None, Some(self.self_signature));
    }

    fn verify_packet(&self, _packet: &PRUDPV1Packet) -> bool {
        true
    }
}
