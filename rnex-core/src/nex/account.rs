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
        Self {
            kerbros_password: passwd.as_bytes().into(),
            username: username.into(),
            pid,
        }
    }

    pub fn new_raw_password(pid: PID, username: &str, passwd: &[u8]) -> Self {
        Self {
            kerbros_password: passwd.into(),
            username: username.into(),
            pid,
        }
    }

    pub fn get_login_data(&self) -> (PID, &[u8]) {
        (self.pid, &self.kerbros_password)
    }
}
