use rnex_base::BaseModule;
#[cfg(feature = "datastore")]
use rnex_ds::DatastoreModule;
#[cfg(feature = "friends")]
use rnex_fpd::FriendsModule;
#[cfg(feature = "match-making")]
use rnex_mm::MatchMakeModule;
#[cfg(feature = "messaging")]
use rnex_msg::MessagingModule;
#[cfg(feature = "ranking")]
use rnex_rk::RankingModule;

use rnex_server::{ConnectionInitData, launch_rnex_module_server};

#[tokio::main]
async fn main() {
    launch_rnex_module_server! {
        ConnectionInitData;
        BaseModule,
        #[cfg(feature="match-making")]
        MatchMakeModule,
        #[cfg(feature="datastore")]
        DatastoreModule,
        #[cfg(feature="friends")]
        FriendsModule,
        #[cfg(feature="ranking")]
        RankingModule,
        #[cfg(feature="messaging")]
        MessagingModule

    }
}
