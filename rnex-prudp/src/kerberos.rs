use std::io::Write;

use bytemuck::{Pod, Zeroable, bytes_of};
use cfg_if::cfg_if;
use hmac::{Hmac, Mac};
use rc4::{KeyInit, Rc4, StreamCipher};
use rnex_util::{PID, date_time::DateTime};
use typenum::Unsigned;

cfg_if! {
    if #[cfg(feature = "friends")]{
        use typenum::U16;
        pub type SessionLengthTy = U16;
    } else {
        use rc4::consts::U32;
        pub type SessionLengthTy = U32;
    }
}
pub const SESSION_KEY_LENGTH: usize = SessionLengthTy::USIZE;

type Md5Hmac = Hmac<md5::Md5>;

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C, packed)]
pub struct TicketInternalData {
    pub issued_time: DateTime,
    pub pid: PID,
    pub session_key: [u8; SESSION_KEY_LENGTH],
}

impl TicketInternalData {
    pub fn new(pid: PID) -> Self {
        Self {
            issued_time: DateTime::now(),
            pid,
            session_key: rand::random(),
        }
    }

    pub fn encrypt(&self, key: [u8; 16]) -> Box<[u8]> {
        let mut data = bytes_of(self).to_vec();

        let mut rc4 = Rc4::new_from_slice(&key).unwrap();
        rc4.apply_keystream(&mut data);

        let mut hmac = <Md5Hmac as KeyInit>::new_from_slice(&key).unwrap();

        hmac.update(&data[..]);

        let hmac_result = &hmac.finalize().into_bytes()[..];

        data.write_all(hmac_result)
            .expect("failed to write data to vec");

        data.into_boxed_slice()
    }
}

#[derive(Pod, Zeroable, Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct Ticket {
    pub session_key: [u8; SESSION_KEY_LENGTH],
    pub pid: PID,
}

impl Ticket {
    pub fn encrypt(&self, key: [u8; 16], internal_data: &[u8]) -> Box<[u8]> {
        let mut data = bytes_of(self).to_vec();

        data.extend_from_slice(bytes_of(&(internal_data.len() as u32)));
        data.extend_from_slice(internal_data);

        let mut rc4 = Rc4::new_from_slice(&key).unwrap();
        rc4.apply_keystream(&mut data);

        let mut hmac = <Md5Hmac as KeyInit>::new_from_slice(&key).unwrap();

        hmac.update(&data[..]);

        let hmac_result = &hmac.finalize().into_bytes()[..];

        data.write_all(hmac_result)
            .expect("failed to write data to vec");

        data.into_boxed_slice()
    }
}
