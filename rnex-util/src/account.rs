use md5::{Digest, Md5};

use crate::PID;

#[derive(Clone, Debug)]
pub struct Account {
    pub pid: PID,
    pub username: String,
    pub nex_key: [u8; 16],
}

impl Account {
    pub fn new(pid: PID, username: &str, passwd: &str) -> Self {
        let iteration_count = 65000 + pid % 1024;
        // we do one iteration out here to ensure the key is always 16 bytes

        let mut key: [u8; 16] = {
            let mut md5 = Md5::new();
            md5.update(passwd);
            md5.finalize().into()
        };

        for _ in 1..iteration_count {
            let mut md5 = Md5::new();
            md5.update(key);
            key = md5.finalize().into();
        }

        Self {
            nex_key: key,
            username: username.into(),
            pid,
        }
    }

    pub fn new_raw_key(pid: PID, username: &str, nex_key: [u8; 16]) -> Self {
        Self {
            username: username.into(),
            pid,
            nex_key,
        }
    }

    pub fn get_login_data(&self) -> (PID, [u8; 16]) {
        (self.pid, self.nex_key)
    }
}
