#[cfg(feature = "nx")]
use rnex_rmc::RmcSerialize;
use rnex_rmc::{method_id, response::ErrorCode, rmc_proto};

#[cfg(feature = "nx")]
#[derive(RmcSerialize, Clone, Debug, Default)]
#[rmc_struct(0)]
pub struct UniqueIdInfo {
    pub id: u64,
    pub password: u64,
}

#[rmc_proto(110)]
pub trait Utility {
    #[method_id(1)]
    async fn acquire_nex_unique_id(&self) -> Result<u64, ErrorCode>;
    #[cfg(feature = "nx")]
    #[method_id(2)]
    async fn acquire_nex_unique_id_with_password(&self) -> Result<UniqueIdInfo, ErrorCode>;
    #[cfg(all(feature = "nx", not(feature = "splatoon2")))]
    #[method_id(3)]
    async fn associate_nex_unique_id_with_my_principal_id(&self, id: u64) -> Result<(), ErrorCode>;
    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    #[method_id(3)]
    async fn associate_nex_unique_id_with_my_principal_id(
        &self,
        info: UniqueIdInfo,
    ) -> Result<(), ErrorCode>;
    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    #[method_id(4)]
    async fn associate_nex_unique_ids_with_my_principal_id(
        &self,
        infos: Vec<UniqueIdInfo>,
    ) -> Result<(), ErrorCode>;
    #[cfg(feature = "nx")]
    #[method_id(5)]
    async fn get_associated_nex_unique_id_with_my_principal_id(
        &self,
    ) -> Result<UniqueIdInfo, ErrorCode>;
    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    #[method_id(6)]
    async fn get_associated_nex_unique_ids_with_my_principal_id(
        &self,
    ) -> Result<Vec<UniqueIdInfo>, ErrorCode>;
    #[method_id(7)]
    async fn get_integer_settings(&self, index: u32) -> Result<Vec<(u16, i32)>, ErrorCode>;
    #[cfg(feature = "nx")]
    #[method_id(8)]
    async fn get_string_settings(&self, index: u32) -> Result<Vec<(u16, String)>, ErrorCode>;
    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    #[method_id(9)]
    async fn acquire_tag_id(&self, nex_unique_ids: Vec<u64>) -> Result<u64, ErrorCode>;
    #[cfg(all(feature = "nx", feature = "splatoon2"))]
    #[method_id(10)]
    async fn update_current_user(&self, param: UpdateCurrentUserParam) -> Result<(), ErrorCode>;
}

#[cfg(all(feature = "nx", feature = "splatoon2"))]
#[derive(RmcSerialize, Clone, Debug)]
#[rmc_struct(0)]
pub struct UpdateCurrentUserParam {
    pub info: UniqueIdInfo,
    pub region: u8,
}
