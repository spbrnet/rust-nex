use rnex_core::{
    PID, executables::common::SECURE_SERVER_ACCOUNT, nex::account::Account,
    prudp::ticket::read_secure_connection_data, rmc::structures::RmcSerialize,
};

use crate::crypto::Crypto;

pub struct Secure(&'static Account);

impl Crypto for Secure {
    fn new_connection(&self, data: &[u8]) -> Option<(PID, Vec<u8>)> {
        let (_, pid, check_value) = read_secure_connection_data(data, &self.0)?;

        let check_value_response = check_value + 1;

        let data = bytemuck::bytes_of(&check_value_response);

        let mut response = Vec::new();

        data.serialize(&mut response).ok()?;

        Some((pid, response))
    }
    fn new() -> Self {
        Self(&SECURE_SERVER_ACCOUNT)
    }
}
