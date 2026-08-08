#![allow(async_fn_in_trait)]

use rnex_rmc::define_rmc_proto;
pub mod auth;
use auth::{Auth, RawAuth, RawAuthInfo, RemoteAuth};

define_rmc_proto!(
    proto AuthProtocol{
        Auth
    }
);
