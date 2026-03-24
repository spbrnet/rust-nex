use rnex_core::PID;

pub mod insecure;
pub mod secure;

pub trait Crypto: 'static + Send + Sync {
    fn new_connection(&self, data: &[u8]) -> Option<(PID, Vec<u8>)>;
    fn new() -> Self;
}
