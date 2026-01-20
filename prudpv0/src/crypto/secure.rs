use crate::crypto::Crypto;

pub struct Secure();

impl Crypto for Secure {
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        todo!()
    }
}
