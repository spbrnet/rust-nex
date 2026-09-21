use hmac::{Hmac, KeyInit, Mac};
use md5::{Digest, Md5};
use thiserror::Error;

use crate::PID;

type HmacMd5 = Hmac<Md5>;

pub fn derive_pid_hmac(pid: PID, key: &str) -> [u8; 16] {
    let mut mac = HmacMd5::new_from_slice(key.as_bytes()).expect("HMAC accepts keys of any size");
    mac.update(&(pid as u32).to_le_bytes());
    mac.finalize().into_bytes().into()
}

#[derive(Clone, Debug)]
pub struct Account {
    pub pid: PID,
    pub username: String,
    pub nex_key: [u8; 16],
}

#[cfg(test)]
mod tests {
    use super::derive_pid_hmac;

    #[test]
    fn pid_hmac_uses_little_endian_pid() {
        assert_eq!(
            hex::encode(derive_pid_hmac(0x1234_5678, "secret")),
            "b1c5e88bab384fcd3460804ef3ff473e"
        );
    }
}

#[derive(Debug, Error)]
pub enum AccountConfigError {
    #[error("missing {0}")]
    MissingEnvironmentVariable(&'static str),
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
    pub fn from_password_env(
        pid: PID,
        username: &str,
        variable: &'static str,
    ) -> Result<Self, AccountConfigError> {
        let password = std::env::var(variable)
            .map_err(|_| AccountConfigError::MissingEnvironmentVariable(variable))?;
        Ok(Self::new(pid, username, &password))
    }
}
