use crate::crypto::Crypto;

pub struct Insecure();

impl Crypto for Insecure {
    fn calculate_checksum(&self, data: &[u8]) -> u8 {
        todo!()
    }
}
