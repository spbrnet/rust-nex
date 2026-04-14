use crate::define_rmc_proto;
use macros::rmc_struct;
use rnex_core::prudp::socket_addr::PRUDPSockAddr;
use std::sync::{Weak};
use rnex_core::PID;
use rnex_core::nex::remote_console::RemoteConsole;
use rnex_core::nex::s3presigner::S3Presigner;
use rnex_core::rmc::response::ErrorCode;
use rnex_core::rmc::protocols::secure::{Secure, RawSecure, RawSecureInfo, RemoteSecure};
use rnex_core::rmc::protocols::datastore::{CompletePostParam, GetMetaInfo, GetMetaParam, KeyValue, RateCustomRankingParam};
use rnex_core::rmc::protocols::datastore::{DataStore, RawDataStore, RawDataStoreInfo, RemoteDataStore, PreparePostParam, ReqPostInfo};
use crate::nex::user::User;

impl DataStore for User {
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode> {
        println!("dataid: {}", metaparam.dataid);
        println!("access password: {}", metaparam.access_password);

        // just trying to see what methods it tries to use
        Err(ErrorCode::DataStore_NotFound)
    }

    async fn prepare_post_object(&self, postparam: PreparePostParam) -> Result<ReqPostInfo, ErrorCode> {
        let data_id: u64 = 9400001;
        let presigner = S3Presigner::new("https://s3.perditum.com", "miku".into()).await;

        let key = format!("data/{}.bin", data_id);

        let (upload_url, fields) = presigner.generate_presigned_post(&key).await;

        let form_fields = fields.into_iter().map(|(k, v)| {
            KeyValue { key: k, value: v }
        }).collect();

        Ok(ReqPostInfo {
            dataid: data_id,
            url: upload_url,
            request_headers: vec![],
            form_fields,
            root_ca_cert: vec![],
        })
    }

    async fn complete_post_object(&self, completeparam: CompletePostParam) -> Result<(), ErrorCode> {
        // whatever
        println!("dataid: {}", completeparam.dataid);
        println!("succeeded?: {}", completeparam.success);

        Ok(())
    }

    async fn rate_custom_ranking(&self, rankingparam: Vec<RateCustomRankingParam>) -> Result<(), ErrorCode> {
        // this returns nothing
        Ok(())
    }
}