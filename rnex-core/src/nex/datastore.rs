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
use rnex_core::executables::common::{RNEX_DATASTORE_S3_BUCKET, RNEX_DATASTORE_S3_ENDPOINT};

impl DataStore for User {
    async fn get_meta(&self, metaparam: GetMetaParam) -> Result<GetMetaInfo, ErrorCode> {
        println!("dataid: {}", metaparam.dataid);
        println!("access password: {}", metaparam.access_password);

        // just trying to see what methods it tries to use
        Err(ErrorCode::DataStore_NotFound)
    }

    async fn prepare_post_object(&self, postparam: PreparePostParam) -> Result<ReqPostInfo, ErrorCode> {
        let data_id: u64 = 9400001;
        let presigner = S3Presigner::new(
            &format!("https://{}", *RNEX_DATASTORE_S3_ENDPOINT),
            format!("{}", *RNEX_DATASTORE_S3_BUCKET)
        ).await;

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

    async fn get_application_config(&self, appid: u32) -> Result<Vec<i32>, ErrorCode> {
        const MAX_COURSE_UPLOADS: i32 = 100;

        let config = match appid {
            0 => vec![
                0x00000001, 0x00000032, 0x00000096, 0x0000012c, 0x000001f4,
                0x00000320, 0x00000514, 0x000007d0, 0x00000bb8, 0x00001388,
                MAX_COURSE_UPLOADS, 0x00000014, 0x0000001e, 0x00000028, 0x00000032,
                0x0000003c, 0x00000046, 0x00000050, 0x0000005a, 0x00000064,
                0x00000023, 0x0000004b, 0x00000023, 0x0000004b, 0x00000032,
                0x00000000, 0x00000003, 0x00000003, 0x00000064, 0x00000006,
                0x00000001, 0x00000060, 0x00000005, 0x00000060, 0x00000000,
                0x000007e4, 0x00000001, 0x00000001, 0x0000000c, 0x00000000,
            ],
            1 => vec![
                2,
                1770179696,
                1770179664,
                1770179640,
                1770180827,
                1770180777,
                1770180745,
                1770177625,
                1770177590,
            ],
            2 => vec![0x000007df, 0x0000000c, 0x00000016, 0x00000005, 0x00000000],
            10 => vec![35, 75, 96, 40, 5, 6],
            _ => {
                log::error!("unknown SMM app id: {}", appid);
                return Err(ErrorCode::DataStore_Unknown);
            }
        };

        Ok(config)
    }
}