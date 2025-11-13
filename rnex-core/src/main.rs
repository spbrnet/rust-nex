#![allow(dead_code)]
#![allow(async_fn_in_trait)]
//#![warn(missing_docs)]

//! # Splatoon RNEX server
//!
//! This server still includes the code for rnex itself as this is the first rnex server and thus
//! also the first and only current usage of rnex, expect this and rnex to be split into seperate
//! repos soon.

extern crate self as rust_nex;

use once_cell::sync::Lazy;
use std::hint::black_box;

mod prudp;
pub mod rmc;
//mod protocols;

mod grpc;
mod kerberos;
mod nex;
mod result;
mod versions;
pub mod reggie;
pub mod util;
pub mod common;

pub mod config{
    pub const FEATURE_HAS_STRUCT_HEADER: bool = cfg!(feature = "rmc_struct_header");
}

use std::io::Cursor;
use std::ops::Deref;
use rnex_core::kerberos::KerberosDateTime;
use rnex_core::rmc::structures::matchmake::{AutoMatchmakeParam, Gathering, MatchmakeParam, MatchmakeSession, MatchmakeSessionSearchCriteria};
use rnex_core::rmc::structures::RmcSerialize;
use rnex_core::rmc::structures::variant::Variant;

static DUMMY: Lazy<AutoMatchmakeParam> = Lazy::new(|| AutoMatchmakeParam{
    additional_participants: vec![1,2,3,4],
    auto_matchmake_option: 10,
    gid_for_participation_check: 9,
    join_message: "hi".to_string(),
    participation_count: 32,
    target_gids: vec![45,2,51,1,1,1,1],
    search_criteria: vec![MatchmakeSessionSearchCriteria{
        attribs: vec!["hi".to_string(), "ig".to_string(), "gotta put data here".to_string()],
        exclude_locked: true,
        exclude_non_host_pid: false,
        exclude_system_password_set: true,
        exclude_user_password_set: false,
        game_mode: "some gamemode".to_string(),
        matchmake_param: MatchmakeParam{
            params: vec![
                ("SR".to_string(), Variant::Bool(true)),
                ("SR2".to_string(), Variant::Double(1.0)),
                ("SR3".to_string(), Variant::SInt64(42)),
                ("SR4".to_string(), Variant::String("test".to_string()))
            ]
        },
        matchmake_system_type: "some type".to_string(),
        maximum_participants: "???".to_string(),
        minimum_participants: "-99".to_string(),
        refer_gid: 123,
        selection_method: 9999999,
        vacant_only: true,
        vacant_participants: 1000
    }],
    matchmake_session: MatchmakeSession{
        refer_gid: 10,
        matchmake_system_type: 139,
        matchmake_param: MatchmakeParam{
            params: vec![
                ("QSR".to_string(), Variant::Bool(false)),
                ("SRQ2".to_string(), Variant::Double(1.1)),
                ("SQR3".to_string(), Variant::SInt64(422)),
                ("SDR4".to_string(), Variant::String("tetst".to_string()))
            ]
        },
        participation_count: 99,
        application_buffer: vec![1,2,3,4,5,6,7,8,9],
        attributes: vec![10,20,99,100000],
        datetime: KerberosDateTime::now(),
        gamemode: 111,
        open_participation: false,
        option0: 100,
        progress_score: 1,
        system_password_enabled: false,
        user_password: "aaa".to_string(),
        session_key: vec![91,123,5,2,1,2,4,124,4],
        user_password_enabled: false,
        gathering: Gathering{
            minimum_participants: 1,
            maximum_participants: 12,
            description: "aaargh".to_string(),
            flags: 100,
            host_pid: 999999919,
            owner_pid: 138830,
            participant_policy: 1,
            policy_argument: 99837,
            self_gid: 129,
            state: 1389488
        }
    }
});

static DUMMY_SER: Lazy<Vec<u8>> = Lazy::new(|| serialize_to_vec(DUMMY.deref()));

fn serialize_to_vec(r: &impl RmcSerialize) -> Vec<u8>{
    let vec = r.to_data();

    vec.unwrap()
}

fn read_struct<T: RmcSerialize>(r: &[u8]) -> T{
    T::deserialize(&mut Cursor::new(r)).unwrap()
}

fn main(){
    for _ in 0..10000000 {
        let v = serialize_to_vec(black_box(DUMMY.deref()));
        let u = read_struct::<AutoMatchmakeParam>(black_box(DUMMY_SER.deref().as_slice()));
        black_box(v);
        black_box(u);
    }
}