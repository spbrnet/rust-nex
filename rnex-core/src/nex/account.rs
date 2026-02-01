use macros::RmcSerialize;

#[derive(RmcSerialize, Clone)]
pub struct Account {
    pub pid: u32,
    pub username: String,
    pub kerbros_password: Box<[u8]>,
}

impl Account {
    pub fn new(pid: u32, username: &str, passwd: &str) -> Self {
        let passwd_data = passwd.as_bytes();

        Self {
            kerbros_password: passwd.as_bytes().into(),
            username: username.into(),
            pid,
        }
    }

    pub fn new_raw_password(pid: u32, username: &str, passwd: &[u8]) -> Self {
        Self {
            kerbros_password: passwd.into(),
            username: username.into(),
            pid,
        }
    }

    pub fn get_login_data(&self) -> (u32, &[u8]) {
        (self.pid, &self.kerbros_password)
    }
}
