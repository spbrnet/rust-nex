use std::process::abort;

use cfg_if::cfg_if;
use tracing::error;

cfg_if! {
    if #[cfg(feature = "prudpv0")]{
        pub use prudpv0::*;
    } else if #[cfg(feature = "prudpv1")] {
        pub use prudpv1_proxy::*;
    }  else if #[cfg(feature = "prudplite")]{
        pub use prudplite::*;
    } else {
        compile_error!("no proxy type has been set");
    }
}

pub fn edge_node_dc_callback() {
    error!("disconnected from node holder, aborting!");
    abort()
}
