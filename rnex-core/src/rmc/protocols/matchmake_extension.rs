use macros::{method_id, rmc_proto};
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::structures::matchmake::{
    AutoMatchmakeParam, CreateMatchmakeSessionParam, JoinMatchmakeSessionParam, MatchmakeSession,
};

use crate::rmc::protocols::notifications::NotificationEvent;
use crate::rmc::structures::any::Any;
use crate::rmc::structures::matchmake::MatchmakeSessionSearchCriteria;

#[rmc_proto(109)]
pub trait MatchmakeExtension {
    #[method_id(1)]
    async fn close_participation(&self, gid: u32) -> Result<(), ErrorCode>;

    #[method_id(2)]
    async fn open_participation(&self, gid: u32) -> Result<(), ErrorCode>;
    #[method_id(6)]
    async fn create_matchmake_session(
        &self,
        gathering: Any,
        message: String,
    ) -> Result<(u32, Vec<u8>), ErrorCode>;

    #[method_id(9)]
    async fn update_notification_data(
        &self,
        ty: u32,
        param1: u32,
        param2: u32,
        str_param: String,
    ) -> Result<(), ErrorCode>;

    #[method_id(10)]
    async fn get_friend_notification_data(
        &self,
        ty: i32,
    ) -> Result<Vec<NotificationEvent>, ErrorCode>;
    #[method_id(15)]
    async fn auto_matchmake_with_search_criteria_postpone(
        &self,
        criteria: Vec<MatchmakeSessionSearchCriteria>,
        gathering: Any,
        join_msg: String,
    ) -> Result<Any, ErrorCode>;

    #[method_id(30)]
    async fn join_matchmake_session_ex(
        &self,
        gid: u32,
        message: String,
        dont_care_block_list: bool,
        // this is to cheat support for v3-3-0
        //participation_count: u16,
    ) -> Result<Vec<u8>, ErrorCode>;

    #[method_id(8)]
    async fn modify_current_game_attribute(
        &self,
        gid: u32,
        attrib_index: u32,
        attrib_val: u32,
    ) -> Result<(), ErrorCode>;

    #[method_id(16)]
    async fn get_playing_session(&self, pids: Vec<u32>) -> Result<Vec<()>, ErrorCode>;

    #[method_id(34)]
    async fn update_progress_score(&self, gid: u32, progress: u8) -> Result<(), ErrorCode>;
    #[method_id(38)]
    async fn create_matchmake_session_with_param(
        &self,
        session: CreateMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode>;

    #[method_id(39)]
    async fn join_matchmake_session_with_param(
        &self,
        session: JoinMatchmakeSessionParam,
    ) -> Result<MatchmakeSession, ErrorCode>;

    #[method_id(40)]
    async fn auto_matchmake_with_param_postpone(
        &self,
        session: AutoMatchmakeParam,
    ) -> Result<MatchmakeSession, ErrorCode>;

    #[method_id(41)]
    async fn find_matchmake_session_by_gathering_id_detail(
        &self,
        gid: u32,
    ) -> Result<MatchmakeSession, ErrorCode>;
}
