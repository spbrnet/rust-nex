use crate::grpc::account::Error::SomethingHappened;
use json::{JsonValue, object};
use nex_account::grpc::Pid;
use nex_account::grpc::nex_account_service_client::NexAccountServiceClient;
use once_cell::sync::Lazy;
use rnex_core::PID;
use std::array::TryFromSliceError;
use std::ops::Deref;
use std::sync::LazyLock;
use std::{env, result};
use thiserror::Error;
use tokio::task::{JoinError, spawn_blocking};
use tonic::transport::Channel;

static API_KEY: Lazy<String> = Lazy::new(|| {
    let key = env::var("ACCOUNT_GQL_API_KEY").expect("no graphql ip specified");

    key
});

static CLIENT_URI: Lazy<String> = Lazy::new(|| {
    env::var("ACCOUNT_GQL_URL")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no graphql ip specified")
});

#[derive(Error, Debug)]
pub enum Error {
    #[error(transparent)]
    RequestError(#[from] ureq::Error),
    #[error(transparent)]
    Json(#[from] json::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status),
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    #[error("invalid password size: {0}")]
    PasswordConversion(#[from] TryFromSliceError),
    #[error("something happened")]
    SomethingHappened,
    #[error("error joining blocking task: {0}")]
    Join(#[from] JoinError),
}

pub type Result<T> = result::Result<T, Error>;

static NEX_ACCOUNT_URL: LazyLock<String> =
    LazyLock::new(|| env::var("NEX_ACCOUNT_ENDPOINT").expect("NEX_ACCOUNT_ENDPOINT not set"));

pub struct Client(NexAccountServiceClient<Channel>); //(reqwest::Client);

impl Client {
    pub async fn new() -> Result<Self> {
        let client = NexAccountServiceClient::connect(NEX_ACCOUNT_URL.as_str()).await?;
        Ok(Self(client))
    }

    pub async fn get_nex_key(&mut self, pid: PID) -> Result<[u8; 16]> {
        let prekey = self.0.get_nex_key_by_pid(Pid { pid }).await?.into_inner();

        println!("{:?}", prekey);

        let nexkey: [u8; 16] = prekey
            .key
            .try_into()
            .map_err(|_| Error::SomethingHappened)?;

        Ok(nexkey)
    }

    pub async fn get_user_level(&mut self, pid: PID) -> Result<i32> {
        // let req = self
        //     .do_request(object! {
        //         "query": r"query($pid: Int!){
        //             userByPid(pid: $pid){
        //                 accountLevel
        //             }
        //         }",
        //         "variables": {
        //             "pid": pid
        //         }
        //     })
        //     .await?;
        //
        // let Some(val) = req
        //     .entries()
        //     .find(|v| v.0 == "data")
        //     .ok_or(SomethingHappened)?
        //     .1
        //     .entries()
        //     .find(|v| v.0 == "userByPid")
        //     .ok_or(SomethingHappened)?
        //     .1
        //     .entries()
        //     .find(|v| v.0 == "accountLevel")
        //     .ok_or(SomethingHappened)?
        //     .1
        //     .as_i32()
        // else {
        //     return Err(SomethingHappened);
        // };

        // everyone is tester until this is implemented
        Ok(0)
    }

    // pub async fn get_pid_from_token(&mut self, token: String) -> Result<PID> {
    //     let req = self
    //         .do_request(object! {
    //             "query":
    //             r"query($token: String!){
    //                 token(tokenData: $token){
    //                     pid
    //                 }
    //             }",
    //             "variables": {
    //                 "token": token
    //             }
    //         })
    //         .await?;
    //     // this breaks switch nex servers and should be fixed eventually
    //     let Some(val) = req
    //         .entries()
    //         .find(|v| v.0 == "data")
    //         .ok_or(SomethingHappened)?
    //         .1
    //         .entries()
    //         .find(|v| v.0 == "token")
    //         .ok_or(SomethingHappened)?
    //         .1
    //         .entries()
    //         .find(|v| v.0 == "pid")
    //         .ok_or(SomethingHappened)?
    //         .1
    //         .as_i32()
    //     else {
    //         return Err(SomethingHappened);
    //     };
    //
    //     Ok(val)
    // }

    /*pub async fn get_user_data(&mut self , pid: u32) -> Result<GetUserDataResponse>{
        let req = Request::new(GetUserDataRequest{
            pid
        });

        let response = self.0.get_user_data(req).await?.into_inner();

        Ok(response)
    }*/
}

/*

pub struct Client(AccountClient<InterceptedService<Channel, InterceptorFunc>>);

impl Client{
    pub async fn new() -> Result<Self>{
        let channel = Channel::from_static(&*CLIENT_URI).connect().await?;

        let func = Box::new(&|mut req: Request<()>|{
            req.metadata_mut().insert("x-api-key", API_KEY.clone());
            Ok(req)
        }) as InterceptorFunc;

        let client = AccountClient::with_interceptor(channel, func);
        Ok(Self(client))
    }

    pub async fn get_nex_password(&mut self , pid: u32) -> Result<[u8; 16]>{
        let req = Request::new(GetNexPasswordRequest{
            pid
        });

        let response = self.0.get_nex_password(req).await?.into_inner();

        Ok(response.password.as_bytes().try_into()?)
    }

    pub async fn get_user_data(&mut self , pid: u32) -> Result<GetUserDataResponse>{
        let req = Request::new(GetUserDataRequest{
            pid
        });

        let response = self.0.get_user_data(req).await?.into_inner();

        Ok(response)
    }
}
*/
