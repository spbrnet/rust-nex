#![allow(dead_code)]
// rnex makes extensive use of async functions in public traits
// this is however fine because these traits should never(and i mean NEVER) be used dynamically
#![allow(async_fn_in_trait)]
//#![warn(missing_docs)]



extern crate self as rnex_core;

pub mod prudp;
pub mod rmc;
//mod protocols;

pub mod grpc;
pub mod kerberos;
pub mod nex;
pub mod result;
pub mod versions;
pub mod web;
pub mod common;
pub mod reggie;
pub mod rnex_proxy_common;
pub mod util;
pub mod executables;
pub use macros::*;
