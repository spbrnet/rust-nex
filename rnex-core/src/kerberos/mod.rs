use bytemuck::{Pod, Zeroable, bytes_of};
use cfg_if::cfg_if;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike, Utc};
use hmac::Hmac;
use hmac::Mac;
use md5::{Digest, Md5};
use rc4::KeyInit;
use rc4::cipher::StreamCipherCoreWrapper;
use rc4::{Rc4, Rc4Core, StreamCipher};
use rnex_core::rmc::structures::RmcSerialize;
use std::io::{Read, Write};
use typenum::U16;
use typenum::Unsigned;

use rnex_core::rmc::structures::Result;

use rnex_core::PID;

cfg_if! {
    if #[cfg(feature = "friends")]{
        pub type SessionLengthTy = U16;
    } else {
        use rc4::consts::U32;
        pub type SessionLengthTy = U32;
    }
}
pub const SESSION_KEY_LENGTH: usize = SessionLengthTy::USIZE;

type Md5Hmac = Hmac<md5::Md5>;

pub fn derive_key(pid: PID, password: &[u8]) -> [u8; 16] {
    let iteration_count = 65000 + pid % 1024;
    // we do one iteration out here to ensure the key is always 16 bytes

    let mut key: [u8; 16] = {
        let mut md5 = Md5::new();
        md5.update(password);
        md5.finalize().try_into().unwrap()
    };

    for _ in 1..iteration_count {
        let mut md5 = Md5::new();
        md5.update(key);
        key = md5.finalize().try_into().unwrap();
    }

    key
}
#[derive(Pod, Zeroable, Copy, Clone, Debug, Eq, PartialEq, Default)]
#[repr(transparent)]
pub struct KerberosDateTime(pub u64);

impl KerberosDateTime {
    pub fn from_i64(val: i64) -> Self {
        Self(val as u64)
    }

    pub fn from_naive(dt: chrono::NaiveDateTime) -> Self {
        use chrono::Datelike;
        use chrono::Timelike;
        Self::new(
            dt.second() as u64,
            dt.minute() as u64,
            dt.hour() as u64,
            dt.day() as u64,
            dt.month() as u64,
            dt.year() as u64,
        )
    }

    pub fn new(second: u64, minute: u64, hour: u64, day: u64, month: u64, year: u64) -> Self {
        Self(second | (minute << 6) | (hour << 12) | (day << 17) | (month << 22) | (year << 26))
    }

    pub fn now() -> Self {
        let now = chrono::Utc::now();
        Self::new(
            now.second() as u64,
            now.minute() as u64,
            now.hour() as u64,
            now.day() as u64,
            now.month() as u64,
            now.year() as u64,
        )
    }

    #[inline]
    pub fn get_seconds(&self) -> u8 {
        (self.0 & 0b111111) as u8
    }

    #[inline]
    pub fn get_minutes(&self) -> u8 {
        ((self.0 >> 6) & 0b111111) as u8
    }
    #[inline]
    pub fn get_hours(&self) -> u8 {
        ((self.0 >> 12) & 0b111111) as u8
    }
    #[inline]
    pub fn get_days(&self) -> u8 {
        ((self.0 >> 17) & 0b111111) as u8
    }

    #[inline]
    pub fn get_month(&self) -> u8 {
        ((self.0 >> 22) & 0b1111) as u8
    }

    #[inline]
    pub fn get_year(&self) -> u64 {
        (self.0 >> 26) & 0xFFFFFFFF
    }

    pub fn to_regular_time(&self) -> chrono::DateTime<Utc> {
        NaiveDateTime::new(
            NaiveDate::from_ymd_opt(
                self.get_year() as i32,
                self.get_month() as u32,
                self.get_days() as u32,
            )
            .unwrap(),
            NaiveTime::from_hms_opt(
                self.get_hours() as u32,
                self.get_minutes() as u32,
                self.get_seconds() as u32,
            )
            .unwrap(),
        )
        .and_utc()
    }
}

impl RmcSerialize for KerberosDateTime {
    fn serialize(&self, writer: &mut impl Write) -> Result<()> {
        Ok(self.0.serialize(writer)?)
    }

    fn deserialize(reader: &mut impl Read) -> Result<Self> {
        Ok(Self(u64::deserialize(reader)?))
    }
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(C, packed)]
pub struct TicketInternalData {
    pub issued_time: KerberosDateTime,
    pub pid: PID,
    pub session_key: [u8; SESSION_KEY_LENGTH],
}

impl TicketInternalData {
    pub(crate) fn new(pid: PID) -> Self {
        Self {
            issued_time: KerberosDateTime::now(),
            pid,
            session_key: rand::random(),
        }
    }

    pub(crate) fn encrypt(&self, key: [u8; 16]) -> Box<[u8]> {
        let mut data = bytes_of(self).to_vec();

        let mut rc4: StreamCipherCoreWrapper<Rc4Core<U16>> = Rc4::new_from_slice(&key).unwrap();
        rc4.apply_keystream(&mut data);

        let mut hmac = <Md5Hmac as KeyInit>::new_from_slice(&key).unwrap();

        hmac.write_all(&data[..])
            .expect("failed to write data to hmac");

        let hmac_result = &hmac.finalize().into_bytes()[..];

        data.write_all(&hmac_result)
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

        internal_data
            .serialize(&mut data)
            .expect("unable to write to vec");

        let mut rc4: StreamCipherCoreWrapper<Rc4Core<U16>> = Rc4::new_from_slice(&key).unwrap();
        rc4.apply_keystream(&mut data);

        let mut hmac = <Md5Hmac as KeyInit>::new_from_slice(&key).unwrap();

        hmac.write_all(&data[..])
            .expect("failed to write data to hmac");

        let hmac_result = &hmac.finalize().into_bytes()[..];

        data.write_all(&hmac_result)
            .expect("failed to write data to vec");

        data.into_boxed_slice()
    }
}

#[cfg(test)]
mod test {
    use crate::kerberos::KerberosDateTime;

    #[test]
    fn kerberos_time_convert_test() {
        let time = KerberosDateTime(135904948834);

        println!("{}", time.to_regular_time().to_rfc2822());
    }
}
