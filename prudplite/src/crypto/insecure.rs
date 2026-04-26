use rnex_core::PID;

use crate::crypto::Crypto;

pub struct Insecure;

impl Crypto for Insecure {
    fn new_connection(&self, _data: &[u8]) -> Option<(PID, Vec<u8>)> {
        Some((100, vec![]))
    }
    fn new() -> Self {
        Self
    }
}
