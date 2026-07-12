#![allow(dead_code)]
// rnex makes extensive use of async functions in public traits
// this is however fine because these traits should never(and i mean NEVER) be used dynamically
#![allow(async_fn_in_trait)]
//#![warn(missing_docs)]

#[cfg(feature = "big_pid")]
pub type PID = i64;
#[cfg(not(feature = "big_pid"))]
pub type PID = i32;

pub use ctor::ctor;

pub mod prudp;
pub mod rmc;
//mod protocols;

pub mod common;
pub mod executables;
pub mod grpc;
pub mod kerberos;
pub mod nex;
pub mod reggie;
pub mod rnex_proxy_common;
pub mod util;
pub mod versions;
