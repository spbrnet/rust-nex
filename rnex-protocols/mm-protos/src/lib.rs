#![allow(async_fn_in_trait)]

use rnex_rmc::define_rmc_proto;

pub mod matchmake;
pub mod matchmake_ext;
pub mod matchmake_extension;
pub mod nat_traversal;
pub mod notifications;
use matchmake::{Matchmake, RawMatchmake, RawMatchmakeInfo, RemoteMatchmake};
use matchmake_ext::{MatchmakeExt, RawMatchmakeExt, RawMatchmakeExtInfo, RemoteMatchmakeExt};
use matchmake_extension::{
    MatchmakeExtension, RawMatchmakeExtension, RawMatchmakeExtensionInfo, RemoteMatchmakeExtension,
};
use nat_traversal::{
    NatTraversal, NatTraversalConsole, RawNatTraversal, RawNatTraversalConsole,
    RawNatTraversalConsoleInfo, RawNatTraversalInfo, RemoteNatTraversal, RemoteNatTraversalConsole,
};
use notifications::{Notification, RawNotification, RawNotificationInfo, RemoteNotification};

define_rmc_proto!(
    proto MatchMakingProtocol{
        MatchmakeExtension,
        MatchmakeExt,
        Matchmake,
        NatTraversal
    }
);
define_rmc_proto!(
    proto MatchMakingClientProtocol{
        Notification,
        NatTraversalConsole,
    }
);
