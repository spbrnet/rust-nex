use hmac::Hmac;
use md5::Md5;

pub const ACCESS_KEY: &str = "ridfebb9";
pub type HmacMd5 = Hmac<Md5>;
