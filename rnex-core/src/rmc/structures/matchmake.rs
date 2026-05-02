use cfg_if::cfg_if;
use macros::RmcSerialize;
use rnex_core::kerberos::KerberosDateTime;
use rnex_core::rmc::structures::variant::Variant;

use rnex_core::PID;

// rmc structure
#[derive(RmcSerialize, Debug, Clone, Default, PartialEq)]
#[rmc_struct(0)]
pub struct Gathering {
    pub self_gid: u32,
    pub owner_pid: PID,
    pub host_pid: PID,
    pub minimum_participants: u16,
    pub maximum_participants: u16,
    pub participant_policy: u32,
    pub policy_argument: u32,
    pub flags: u32,
    pub state: u32,
    pub description: String,
}

// rmc structure
#[derive(RmcSerialize, Debug, Clone, Default, PartialEq)]
#[rmc_struct(0)]
pub struct MatchmakeParam {
    pub params: Vec<(String, Variant)>,
}

cfg_if! {
    if #[cfg(feature = "v3-5-0")]{
        #[derive(RmcSerialize, Debug, Clone, Default, PartialEq)]
        #[rmc_struct(3)]
        pub struct MatchmakeSession {
            //inherits from
            #[extends]
            pub gathering: Gathering,

            pub gamemode: u32,
            pub attributes: Vec<u32>,
            pub open_participation: bool,
            pub matchmake_system_type: u32,
            pub application_buffer: Vec<u8>,
            pub participation_count: u32,
            pub progress_score: u8,
            pub session_key: Vec<u8>,
            pub option0: u32,
            pub matchmake_param: MatchmakeParam,
            pub datetime: KerberosDateTime,
            pub user_password: String,
            pub refer_gid: u32,
            pub user_password_enabled: bool,
            pub system_password_enabled: bool,
        }
    } else {
        #[derive(RmcSerialize, Debug, Clone, Default, PartialEq)]
        #[rmc_struct(0)]
        pub struct MatchmakeSession {
            //inherits from
            #[extends]
            pub gathering: Gathering,

            pub gamemode: u32,
            pub attributes: Vec<u32>,
            pub open_participation: bool,
            pub matchmake_system_type: u32,
            pub application_buffer: Vec<u8>,
            pub participation_count: u32,
            pub session_key: Vec<u8>,
        }
    }
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(3)]
pub struct MatchmakeSessionSearchCriteria {
    pub attribs: Vec<String>,
    pub game_mode: String,
    pub minimum_participants: String,
    pub maximum_participants: String,
    pub matchmake_system_type: String,
    pub vacant_only: bool,
    pub exclude_locked: bool,
    pub exclude_non_host_pid: bool,
    pub selection_method: u32,
    pub vacant_participants: u16,
    pub matchmake_param: MatchmakeParam,
    pub exclude_user_password_set: bool,
    pub exclude_system_password_set: bool,
    pub refer_gid: u32,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct AutoMatchmakeParam {
    pub matchmake_session: MatchmakeSession,
    pub additional_participants: Vec<PID>,
    pub gid_for_participation_check: u32,
    pub auto_matchmake_option: u32,
    pub join_message: String,
    pub participation_count: u16,
    pub search_criteria: Vec<MatchmakeSessionSearchCriteria>,
    pub target_gids: Vec<u32>,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct CreateMatchmakeSessionParam {
    pub matchmake_session: MatchmakeSession,
    pub additional_participants: Vec<PID>,
    pub gid_for_participation_check: u32,
    pub create_matchmake_session_option: u32,
    pub join_message: String,
    pub participation_count: u16,
}

#[derive(RmcSerialize, Debug, Clone)]
#[rmc_struct(0)]
pub struct MatchmakeBlockListParam {
    option_flag: u32,
}

cfg_if! {
    if #[cfg(feature = "v3-10-22")] {
        #[derive(RmcSerialize, Debug, Clone)]
        #[rmc_struct(1)]
        pub struct JoinMatchmakeSessionParam {
            pub gid: u32,
            pub additional_participants: Vec<PID>,
            pub gid_for_participation_check: u32,
            pub join_matchmake_session_open: u32,
            pub join_matchmake_session_behavior: u8,
            pub user_password: String,
            pub system_password: String,
            pub join_message: String,
            pub participation_count: u16,
            pub extra_participant: u16,
            //pub block_list_param: MatchmakeBlockListParam
        }
    } else {
        #[derive(RmcSerialize, Debug, Clone)]
        #[rmc_struct(0)]
        pub struct JoinMatchmakeSessionParam {
            pub gid: u32,
            pub additional_participants: Vec<PID>,
            pub gid_for_participation_check: u32,
            pub join_matchmake_session_open: u32,
            pub join_matchmake_session_behavior: u8,
            pub user_password: String,
            pub system_password: String,
            pub join_message: String,
            pub participation_count: u16,
            //pub extra_participant: u16,
            //pub block_list_param: MatchmakeBlockListParam
        }
    }
}

pub mod gathering_flags {
    pub const PERSISTENT_GATHERING: u32 = 0x1;
    pub const DISCONNECT_CHANGE_OWNER: u32 = 0x10;
    pub const PERSISTENT_GATHERING_LEAVE_PARTICIPATION: u32 = 0x40;
    pub const PERSISTENT_GATHERING_ALLOW_ZERO_USERS: u32 = 0x80;
    pub const PARTICIPANTS_CHANGE_OWNER: u32 = 0x200;
    pub const VERBOSE_PARTICIPANTS: u32 = 0x400;
    pub const VERBOSE_PARTICIPANTS_EX: u32 = 0x800;
}
