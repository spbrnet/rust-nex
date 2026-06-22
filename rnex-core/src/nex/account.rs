use md5::{Digest, Md5};
use macros::RmcSerialize;

use rnex_core::PID;

#[derive(RmcSerialize, Clone)]
pub struct Account {
    pub pid: PID,
    pub username: String,
    pub kerbros_password: Box<[u8]>,
}

impl Account {
    pub fn new(pid: PID, username: &str, passwd: &str) -> Self {
        let iteration_count = 65000;
        // we do one iteration out here to ensure the key is always 16 bytes

        let mut key: [u8; 16] = {
            let mut md5 = Md5::new();
            md5.update(passwd);
            md5.finalize().try_into().unwrap()
        };

        for _ in 1..iteration_count {
            let mut md5 = Md5::new();
            md5.update(key);
            key = md5.finalize().try_into().unwrap();
        }

        Self {
            kerbros_password: key.into(),
            username: username.into(),
            pid,
        }
    }

    pub fn new_raw_password(pid: PID, username: &str, passwd: &[u8]) -> Self {
        let iteration_count = 65000;
        // we do one iteration out here to ensure the key is always 16 bytes

        let mut key: [u8; 16] = {
            let mut md5 = Md5::new();
            md5.update(passwd);
            md5.finalize().try_into().unwrap()
        };

        for _ in 1..iteration_count {
            let mut md5 = Md5::new();
            md5.update(key);
            key = md5.finalize().try_into().unwrap();
        }

        Self {
            kerbros_password: key.into(),
            username: username.into(),
            pid,
        }
    }

    pub fn get_login_data(&self) -> (PID, &[u8]) {
        (self.pid, &self.kerbros_password)
    }
}
