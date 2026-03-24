#![allow(dead_code)]
// rnex makes extensive use of async functions in public traits
// this is however fine because these traits should never(and i mean NEVER) be used dynamically
#![allow(async_fn_in_trait)]
//#![warn(missing_docs)]

#[cfg(feature = "big_pid")]
pub type PID = u64;
#[cfg(not(feature = "big_pid"))]
pub type PID = u32;

extern crate self as rnex_core;

pub mod prudp;
pub mod rmc;
//mod protocols;

pub mod common;
pub mod executables;
pub mod grpc;
pub mod kerberos;
pub mod nex;
pub mod reggie;
pub mod result;
pub mod rnex_proxy_common;
pub mod util;
pub mod versions;
pub use macros::*;

pub mod config {
    pub const FEATURE_HAS_STRUCT_HEADER: bool = cfg!(feature = "rmc_struct_header");
}
