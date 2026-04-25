use hmac::Hmac;
use md5::Md5;
use proxy_common::RNEX_ACCESS_KEY;

pub const ACCESS_KEY: &str = RNEX_ACCESS_KEY;
pub type HmacMd5 = Hmac<Md5>;
