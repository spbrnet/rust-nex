use crate::prudp::packet::PRUDPV1Packet;
use crate::prudp::socket::{CryptoHandler, CryptoHandlerConnectionInstance};
use hmac::digest::consts::U32;
use log::error;
use rc4::cipher::StreamCipherCoreWrapper;
use rc4::consts::U16;
use rc4::{KeyInit, Rc4, Rc4Core, StreamCipher};
use rnex_core::kerberos::{TicketInternalData, derive_key};
use rnex_core::nex::account::Account;
use rnex_core::prudp::encryption::EncryptionPair;
use rnex_core::rmc::structures::RmcSerialize;
use std::io::Cursor;
use typenum::U5;
use v_byte_helpers::{IS_BIG_ENDIAN, ReadExtensions};

pub fn read_secure_connection_data(data: &[u8], act: &Account) -> Option<([u8; 32], u32, u32)> {
    let mut cursor = Cursor::new(data);

    let mut ticket_data: Vec<u8> = Vec::deserialize(&mut cursor).ok()?;
    let mut request_data: Vec<u8> = Vec::deserialize(&mut cursor).ok()?;

    let ticket_data_size = ticket_data.len();

    let ticket_data = &mut ticket_data[0..ticket_data_size - 0x10];

    let server_key = derive_key(act.pid, act.kerbros_password);

    let mut rc4: StreamCipherCoreWrapper<Rc4Core<U16>> =
        Rc4::new_from_slice(&server_key).expect("unable to init rc4 keystream");

    rc4.apply_keystream(ticket_data);

    let ticket_data: &TicketInternalData = match bytemuck::try_from_bytes(ticket_data) {
        Ok(v) => v,
        Err(e) => {
            error!("unable to read internal ticket data: {}", e);
            return None;
        }
    };

    // todo: add ticket expiration

    let TicketInternalData {
        session_key,
        pid: ticket_source_pid,
        issued_time,
    } = *ticket_data;

    // todo: add checking if tickets are signed with a valid md5-hmac
    let request_data_length = request_data.len();
    let request_data = &mut request_data[0..request_data_length - 0x10];

    let mut rc4: StreamCipherCoreWrapper<Rc4Core<U32>> =
        Rc4::new_from_slice(&session_key).expect("unable to init rc4 keystream");

    rc4.apply_keystream(request_data);

    let mut reqest_data_cursor = Cursor::new(request_data);

    let pid: u32 = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;

    if pid != ticket_source_pid {
        let ticket_created_on = issued_time.to_regular_time();

        error!(
            "someone tried to spoof their pid, ticket was created on: {}",
            ticket_created_on.to_rfc2822()
        );
        return None;
    }

    let _cid: u32 = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;
    let response_check: u32 = reqest_data_cursor.read_struct(IS_BIG_ENDIAN).ok()?;

    Some((session_key, pid, response_check))
}

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
    remote_signature: [u8; 16],
    pid: u32,
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

    fn get_user_id(&self) -> u32 {
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
