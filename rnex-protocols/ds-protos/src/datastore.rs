use rnex_base_protos::ResultsRange;
use rnex_rmc::{
    RmcSerialize, method_id,
    qbuffer::QBuffer,
    qresult::QResult,
    response::ErrorCode,
    rmc_proto,
    util::{PID, date_time::DateTime},
};

#[cfg(feature = "splatoon2")]
#[derive(Clone, Debug)]
pub struct Splatoon2Payload(pub Vec<u8>);

#[cfg(feature = "splatoon2")]
impl rnex_rmc::serialization::RmcSerialize for Splatoon2Payload {
    fn serialize(
        &self,
        writer: &mut (impl std::io::Write + ?Sized),
    ) -> rnex_rmc::serialization::Result<()> {
        writer.write_all(&self.0)?;
        Ok(())
    }

    fn deserialize(
        reader: &mut (impl std::io::Read + ?Sized),
    ) -> rnex_rmc::serialization::Result<Self> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)?;
        Ok(Self(bytes))
    }
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct CoconutViolation {
    pub data_id: u64,
    pub category_code: String,
    pub reason: String,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct CoconutMeta {
    pub data_id: u64,
    pub tweet_id: u64,
    pub region: u8,
    pub post_type: u8,
    pub theme_id: u8,
    pub festival_id: u32,
    pub language: String,
    pub binary_data: QBuffer,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct CoconutGetInfo {
    pub info: DataStoreReqGetInfo,
    pub meta: CoconutMeta,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct CoconutGetParam {
    pub unique_ids: Vec<u64>,
    pub get_type: u8,
    pub region: u8,
    pub festival_id: u32,
}

#[cfg(feature = "splatoon2")]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct OrderedInfo {
    pub gear_kind: i32,
    pub gear_id: i32,
    pub skill_id: i32,
    pub price: i32,
}

#[derive(RmcSerialize, Clone, Debug, Default)]
#[rmc_struct(0)]
pub struct PersistenceTarget {
    pub owner: PID,
    pub persistence_slot_id: u16,
}

#[derive(RmcSerialize, Clone, Debug, Default)]
#[rmc_struct(0)]
pub struct Permission {
    pub permission: u8,
    pub recipient_ids: Vec<PID>,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct RatingInfoWithSlot {
    pub slot: i8,
    pub rating: RatingInfo,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct RatingInfo {
    pub total_value: i64,
    pub count: u32,
    pub initial_value: i64,
}

#[derive(RmcSerialize, Clone, Default)]
#[rmc_struct(0)]
pub struct GetMetaParam {
    pub dataid: i64,
    pub persistence_target: PersistenceTarget,
    pub result_option: u8,
    pub access_password: i64,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct GetMetaInfo {
    pub dataid: i64,
    pub owner: PID,
    pub size: u32,
    pub name: String,
    pub data_type: u16,
    pub meta_binary: QBuffer,
    pub permission: Permission,
    pub del_permission: Permission,
    pub created_time: DateTime,
    pub updated_time: DateTime,
    pub period: u16,
    pub status: u8,
    pub referred_count: u32,
    pub refer_dat_id: u32,
    pub flag: u32,
    pub referred_time: DateTime,
    pub expire_time: DateTime,
    pub tags: Vec<String>,
    pub ratings: Vec<RatingInfoWithSlot>,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct RatingInitParam {
    pub flag: u8,
    pub internal_flag: u8,
    pub lock_type: u8,
    pub initial_value: i64,
    pub range_min: i32,
    pub range_max: i32,
    pub period_hour: i8,
    pub period_duration: i16,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct RatingInitParamWithSlot {
    pub slot: i8,
    pub param: RatingInitParam,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct PersistenceInitParam {
    pub persistence_slot_id: u16,
    pub delete_last_object: bool,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct PreparePostParam {
    pub size: i32,
    pub name: String,
    pub data_type: i16,
    pub meta_binary: QBuffer,
    pub permission: Permission,
    pub del_permission: Permission,
    pub flag: i32,
    pub period: i16,
    pub refer_data_id: i32,
    pub tags: Vec<String>,
    pub rating_init_params: Vec<RatingInitParamWithSlot>,
    pub persistence_init_param: PersistenceInitParam,
    pub extra_data: Vec<String>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct ReqPostInfo {
    pub dataid: i64,
    pub url: String,
    pub request_headers: Vec<KeyValue>,
    pub form_fields: Vec<KeyValue>,
    pub root_ca_cert: Vec<u8>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct CompletePostParam {
    pub dataid: i64,
    pub success: bool,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct RateCustomRankingParam {
    pub dataid: i64,
    pub appid: u32,
    pub score: u32,
    pub period: u16,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct BufferQueueParam {
    pub dataid: i64,
    pub slot: i32,
}

// I just realized I forgot to add "DataStore" in front of the structs. I can't be assed to change it, sucks to be you lol.

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreGetCustomRankingByDataIDParam {
    pub application_id: u32,
    pub data_id_list: Vec<i64>,
    pub result_option: u8,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreCustomRankingResult {
    pub order: u32,
    pub score: u32,
    pub meta_info: GetMetaInfo,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStorePrepareGetParam {
    pub dataid: i64,
    pub lockid: u32,
    pub persistence_target: PersistenceTarget,
    pub access_password: i64,
    pub extra_data: Vec<String>,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreReqGetInfo {
    pub url: String,
    pub request_headers: Vec<KeyValue>,
    pub size: u32,
    pub root_ca_cert: Vec<u8>,
    pub dataid: i64,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(1)]
pub struct DataStoreSearchParam {
    pub search_target: u8,
    pub owner_ids: Vec<PID>,
    pub owner_type: u8,
    pub destination_ids: Vec<i64>,
    pub data_type: u16,
    pub created_after: DateTime,
    pub created_before: DateTime,
    pub updated_after: DateTime,
    pub updated_before: DateTime,
    pub refer_dat_id: u32,
    pub tags: Vec<String>,
    pub result_order_column: u8,
    pub result_order: u8,
    pub result_range: ResultsRange,
    pub result_option: u8,
    pub minimal_rating_frequency: u32,
    pub use_cache: bool,
}

#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct AttachFileParam {
    pub post_param: PreparePostParam,
    pub refer_data_id: i64,
    pub content_type: String,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreRatingTarget {
    pub dataid: i64,
    pub slot: i8,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreRateObjectParam {
    pub rating_value: i32,
    pub access_password: i64,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct DataStoreChangeMetaCompareParam {
    pub comparison_flag: u32,
    pub name: String,
    pub permission: Permission,
    pub del_permission: Permission,
    pub period: u16,
    pub meta_binary: QBuffer,
    pub tags: Vec<String>,
    pub referred_cnt: u32,
    pub data_type: u16,
    pub status: u8,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct DataStoreChangeMetaParam {
    pub dataid: i64,
    pub modifies_flag: u32,
    pub name: String,
    pub permission: Permission,
    pub del_permission: Permission,
    pub period: u16,
    pub meta_binary: QBuffer,
    pub tags: Vec<String>,
    pub update_password: i64,
    pub referred_cnt: u32,
    pub data_type: u16,
    pub status: u8,
    pub compare_param: DataStoreChangeMetaCompareParam,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct DataStoreUploadCourseRecordParam {
    pub dataid: i64,
    pub slot: u8,
    pub score: i32,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct DataStoreGetCourseRecordParam {
    pub dataid: i64,
    pub slot: u8,
}

#[derive(RmcSerialize, Clone, Default, Debug)]
#[rmc_struct(0)]
pub struct DataStoreGetCourseRecordResult {
    pub dataid: i64,
    pub slot: u8,
    pub first_pid: u32,
    pub best_pid: u32,
    pub best_score: i32,
    pub created_time: DateTime,
    pub updated_time: DateTime,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreFileServerObjectInfo {
    pub dataid: i64,
    pub get_info: DataStoreReqGetInfo,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreReportCourseParam {
    pub dataid: i64,
    pub mii_name: String,
    pub report_category: i8,
    pub report_reason: String,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreDeleteParam {
    pub dataid: i64,
    pub update_password: i64,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreCustomRankingRatingCondition {
    pub slot: i8,
    pub min_value: i32,
    pub max_value: i32,
}

#[derive(RmcSerialize, Clone)]
#[rmc_struct(0)]
pub struct DataStoreGetCustomRankingParam {
    pub application_id: u32,
    pub condition: DataStoreCustomRankingRatingCondition,
    pub result_option: u8,
    pub result_range: ResultsRange,
}

#[rmc_proto(115)]
pub trait DataStore {
    #[method_id(47)]
    #[cfg(feature = "splatoon2")]
    async fn coconut_register_meta(&self, meta: CoconutMeta, url: String) -> Result<(), ErrorCode>;
    #[method_id(48)]
    #[cfg(feature = "splatoon2")]
    async fn coconut_rate_post(&self, data_id: u64) -> Result<(), ErrorCode>;
    #[method_id(49)]
    #[cfg(feature = "splatoon2")]
    async fn coconut_get_object_infos(
        &self,
        param: CoconutGetParam,
    ) -> Result<(Vec<CoconutGetInfo>, Vec<QResult>), ErrorCode>;
    #[method_id(50)]
    #[cfg(feature = "splatoon2")]
    async fn coconut_report_violation(&self, violation: CoconutViolation) -> Result<(), ErrorCode>;
    #[method_id(55)]
    #[cfg(feature = "splatoon2")]
    async fn get_ordered_gear(&self) -> Result<OrderedInfo, ErrorCode>;
    #[method_id(56)]
    #[cfg(feature = "splatoon2")]
    async fn purchase_gear(&self, info: OrderedInfo) -> Result<(), ErrorCode>;
    #[method_id(58)]
    #[cfg(feature = "splatoon2")]
    async fn coconut_register_meta_by_param(
        &self,
        meta: CoconutMeta,
        sns_name: String,
        post_id: String,
    ) -> Result<(), ErrorCode>;
    #[method_id(51)]
    #[cfg(feature = "splatoon2")]
    async fn upload_regular_match_result(&self, payload: Splatoon2Payload)
    -> Result<(), ErrorCode>;
    #[method_id(52)]
    #[cfg(feature = "splatoon2")]
    async fn upload_gachi_match_result(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(53)]
    #[cfg(feature = "splatoon2")]
    async fn upload_league_match_result(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(54)]
    #[cfg(feature = "splatoon2")]
    async fn upload_fes_match_result(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(57)]
    #[cfg(feature = "splatoon2")]
    async fn upload_time_attack(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(59)]
    #[cfg(feature = "splatoon2")]
    async fn upload_fes_match_result_v2(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(60)]
    #[cfg(feature = "splatoon2")]
    async fn upload_x_match_result(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(61)]
    #[cfg(feature = "splatoon2")]
    async fn upload_regular_match_result_v2(
        &self,
        payload: Splatoon2Payload,
    ) -> Result<(), ErrorCode>;
    #[method_id(62)]
    #[cfg(feature = "splatoon2")]
    async fn upload_gachi_match_result_v2(
        &self,
        payload: Splatoon2Payload,
    ) -> Result<(), ErrorCode>;
    #[method_id(63)]
    #[cfg(feature = "splatoon2")]
    async fn upload_league_match_result_v2(
        &self,
        payload: Splatoon2Payload,
    ) -> Result<(), ErrorCode>;
    #[method_id(64)]
    #[cfg(feature = "splatoon2")]
    async fn upload_fes_match_result_v3(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(65)]
    #[cfg(feature = "splatoon2")]
    async fn upload_x_match_result_v2(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(66)]
    #[cfg(feature = "splatoon2")]
    async fn prepare_post_play_log(
        &self,
        payload: Splatoon2Payload,
    ) -> Result<ReqPostInfo, ErrorCode>;
    #[method_id(67)]
    #[cfg(feature = "splatoon2")]
    async fn prepare_get_play_log(
        &self,
        payload: Splatoon2Payload,
    ) -> Result<DataStoreReqGetInfo, ErrorCode>;
    #[method_id(68)]
    #[cfg(feature = "splatoon2")]
    async fn upload_fes_match_result_v4(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(69)]
    #[cfg(feature = "splatoon2")]
    async fn upload_coop_result(&self, payload: Splatoon2Payload) -> Result<(), ErrorCode>;
    #[method_id(4)]
    async fn delete_object(&self, param: DataStoreDeleteParam) -> Result<(), ErrorCode>;
    #[method_id(8)]
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode>;
    #[method_id(24)]
    async fn prepare_post_object(
        &self,
        postparam: PreparePostParam,
    ) -> Result<ReqPostInfo, ErrorCode>;
    #[method_id(25)]
    async fn prepare_get_object(
        &self,
        prepare_get_param: DataStorePrepareGetParam,
    ) -> Result<DataStoreReqGetInfo, ErrorCode>;
    #[method_id(26)]
    async fn complete_post_object(&self, completeparam: CompletePostParam)
    -> Result<(), ErrorCode>;
    #[method_id(36)]
    async fn get_metas_multiple_param(
        &self,
        params: Vec<GetMetaParam>,
    ) -> Result<(Vec<GetMetaInfo>, Vec<QResult>), ErrorCode>;
    #[method_id(48)]
    #[cfg(not(feature = "splatoon2"))]
    async fn rate_custom_ranking(
        &self,
        rankingparam: Vec<RateCustomRankingParam>,
    ) -> Result<(), ErrorCode>;
    #[method_id(61)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_application_config(&self, appid: u32) -> Result<Vec<i32>, ErrorCode>;
    #[method_id(49)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_custom_ranking(
        &self,
        param: DataStoreGetCustomRankingParam,
    ) -> Result<(Vec<DataStoreCustomRankingResult>, Vec<QResult>), ErrorCode>;
    #[method_id(50)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_custom_ranking_by_data_id(
        &self,
        custom_ranking_param: DataStoreGetCustomRankingByDataIDParam,
    ) -> Result<(Vec<DataStoreCustomRankingResult>, Vec<QResult>), ErrorCode>;
    #[method_id(53)]
    #[cfg(not(feature = "splatoon2"))]
    async fn add_to_buffer_queues(
        &self,
        bufferparam: Vec<BufferQueueParam>,
        buffers: Vec<QBuffer>,
    ) -> Result<Vec<QResult>, ErrorCode>;
    #[method_id(54)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_buffer_queue(
        &self,
        bufferparam: BufferQueueParam,
    ) -> Result<Vec<QBuffer>, ErrorCode>;
    #[method_id(65)]
    #[cfg(not(feature = "splatoon2"))]
    async fn followings_latest_course_search_object(
        &self,
        course_search_param: DataStoreSearchParam,
        extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode>;
    #[method_id(66)]
    #[cfg(not(feature = "splatoon2"))]
    async fn recommended_course_search_object(
        &self,
        course_search_param: DataStoreSearchParam,
        extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode>;
    #[method_id(74)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_application_config_string(
        &self,
        application_id: u32,
    ) -> Result<Vec<String>, ErrorCode>;
    #[method_id(38)]
    async fn change_meta(&self, param: DataStoreChangeMetaParam) -> Result<(), ErrorCode>;
    #[method_id(40)]
    async fn rate_objects(
        &self,
        targets: Vec<DataStoreRatingTarget>,
        params: Vec<DataStoreRateObjectParam>,
        _transactional: bool,
        fetch_ratings: bool,
    ) -> Result<(Vec<RatingInfo>, Vec<QResult>), ErrorCode>;
    #[method_id(45)]
    async fn get_object_infos(
        &self,
        dataids: Vec<i64>,
    ) -> Result<Vec<DataStoreFileServerObjectInfo>, ErrorCode>;
    #[method_id(57)]
    #[cfg(not(feature = "splatoon2"))]
    async fn complete_attach_file(
        &self,
        complete_attach_param: CompletePostParam,
    ) -> Result<String, ErrorCode>;
    #[method_id(59)]
    #[cfg(not(feature = "splatoon2"))]
    async fn prepare_attach_file(
        &self,
        attach_file_param: AttachFileParam,
    ) -> Result<ReqPostInfo, ErrorCode>;
    #[method_id(71)]
    #[cfg(not(feature = "splatoon2"))]
    async fn upload_course_record(
        &self,
        upload_course_record_param: DataStoreUploadCourseRecordParam,
    ) -> Result<(), ErrorCode>;
    #[method_id(72)]
    #[cfg(not(feature = "splatoon2"))]
    async fn get_course_record(
        &self,
        get_course_record_param: DataStoreGetCourseRecordParam,
    ) -> Result<DataStoreGetCourseRecordResult, ErrorCode>;
    #[method_id(79)]
    #[cfg(not(feature = "splatoon2"))]
    async fn check_rate_custom_ranking_counter(
        &self,
        application_id: u32,
    ) -> Result<bool, ErrorCode>;
    #[method_id(82)]
    #[cfg(not(feature = "splatoon2"))]
    async fn ctr_pickup_course_search_object(
        &self,
        course_search_param: DataStoreSearchParam,
        extra_data: Vec<String>,
    ) -> Result<Vec<DataStoreCustomRankingResult>, ErrorCode>;
    #[method_id(87)]
    #[cfg(not(feature = "splatoon2"))]
    async fn report_course(
        &self,
        report_course_param: DataStoreReportCourseParam,
    ) -> Result<(), ErrorCode>;
}
