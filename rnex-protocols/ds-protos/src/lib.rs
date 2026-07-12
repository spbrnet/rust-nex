#![allow(async_fn_in_trait)]

pub mod datastore;
use datastore::{DataStore, RawDataStore, RawDataStoreInfo, RemoteDataStore};
use rnex_rmc::define_rmc_proto;

define_rmc_proto!(
    proto DatastoreProtocol{
        DataStore
    }
);
