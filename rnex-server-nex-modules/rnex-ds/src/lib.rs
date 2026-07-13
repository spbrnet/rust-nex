#![cfg(feature = "datastore")]
use std::env;

use rnex_server::{ConnectionInitData, RnexManager, RnexModule};
use sqlx::PgPool;
use thiserror::Error;

use crate::{datastore::DatastoreUser, s3presigner::S3Presigner};

pub mod datastore;
pub(crate) mod s3presigner;

#[derive(Debug)]
pub struct DatastoreManager {
    db_pool: PgPool,
    s3_presigner: S3Presigner,
}
pub struct DatastoreModule;
impl RnexManager for DatastoreManager {
    type User = DatastoreUser;
    type InitData = ConnectionInitData;
    async fn init_new_user(
        this: rnex_server::PassthroughInitModule<Self>,
        mod_holder: &rnex_server::ModuleHolder,
        _: &rnex_rmc::RmcConnection,
        _: &Self::InitData,
        _: rnex_server::WeakPassthroughInitModule<Self::User>,
    ) -> Self::User {
        DatastoreUser {
            dm: this,
            base: mod_holder
                .get_ref_init_pt()
                .expect("datastore module cannot work without base module"),
        }
    }
}

#[derive(Error, Debug)]
pub enum ModuleInitError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Env(#[from] env::VarError),
}

impl RnexModule for DatastoreModule {
    type Manager = DatastoreManager;
    type InitError = ModuleInitError;

    async fn create_manager(
        mod_holder: &rnex_server::ModuleHolder,
    ) -> Result<Self::Manager, Self::InitError> {
        Ok(DatastoreManager {
            db_pool: PgPool::connect(&env::var("RNEX_DATASTORE_DATABASE")?).await?,
            s3_presigner: S3Presigner::new(
                env::var("RNEX_DATASTORE_S3_ENDPOINT")?
                    .trim_end_matches('/')
                    .to_string(),
                env::var("RNEX_DATASTORE_S3_BUCKET")?,
            ),
        })
    }
}
