use rnex_base::BaseModule;
#[cfg(feature = "match-making")]
use rnex_mm::MatchMakeModule;
use rnex_server::{ConnectionInitData, launch_rnex_module_server};

#[tokio::main]
async fn main() {
    launch_rnex_module_server! {
        ConnectionInitData;
        BaseModule,
        #[cfg(feature="match-making")]
        MatchMakeModule

    }
}
