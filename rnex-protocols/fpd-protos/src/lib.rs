#![allow(async_fn_in_trait)]

use rnex_rmc::define_rmc_proto;
pub mod account_management;
pub mod friends_3ds;
pub mod friends_wiiu;
pub mod nintendo_notification;
use account_management::{
    AccountManagement, RawAccountManagement, RawAccountManagementInfo, RemoteAccountManagement,
};
use friends_3ds::{Friends3DS, RawFriends3DS, RawFriends3DSInfo, RemoteFriends3DS};
use friends_wiiu::{FriendsWiiU, RawFriendsWiiU, RawFriendsWiiUInfo, RemoteFriendsWiiU};
use nintendo_notification::{
    NintendoNotification, RawNintendoNotification, RawNintendoNotificationInfo,
    RemoteNintendoNotification,
};

define_rmc_proto!(
    proto FriendsUser{
        FriendsWiiU,
        Friends3DS
    }
);
define_rmc_proto!(
    proto FriendRemote{
        NintendoNotification
    }
);
define_rmc_proto!(
    proto FriendsGuest{
        AccountManagement
    }
);
