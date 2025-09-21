use std::{env, result};
use std::array::TryFromSliceError;
use std::str::FromStr;
use json::{object, JsonValue};
use once_cell::sync::Lazy;
use reqwest::{Body, Method, Url};
use reqwest::header::HeaderValue;
use thiserror::Error;
use crate::grpc::account::Error::SomethingHappened;
static API_KEY: Lazy<String> = Lazy::new(||{
    let key = env::var("ACCOUNT_GQL_API_KEY")
        .expect("no graphql ip specified");

    key
});

static CLIENT_URI: Lazy<String> = Lazy::new(||{
    env::var("ACCOUNT_GQL_URL")
        .ok()
        .and_then(|s| s.parse().ok())
        .expect("no graphql ip specified")
});



#[derive(Error, Debug)]
pub enum Error{
    #[error(transparent)]
    Creation(#[from] reqwest::Error),
    #[error(transparent)]
    Json(#[from] json::Error),
    #[error(transparent)]
    Status(#[from] tonic::Status),
    #[error("invalid password size: {0}")]
    PasswordConversion(#[from] TryFromSliceError),
    #[error("something happened")]
    SomethingHappened
}

pub type Result<T> = result::Result<T, Error>;

pub struct Client(reqwest::Client);

impl Client{
    pub async fn new() -> Result<Self> {
        Ok(Self(reqwest::ClientBuilder::new().build()?))
    }

    async fn do_request(&self, request_data: JsonValue) -> Result<JsonValue>{
        let mut request = reqwest::Request::new(Method::POST, Url::from_str(CLIENT_URI.as_str()).unwrap());

        *(request.body_mut()) = Some(Body::from(request_data.to_string()));
        request.headers_mut().insert("X-API-Key", HeaderValue::from_str(&API_KEY).unwrap());
        request.headers_mut().insert("Content-Type", HeaderValue::from_str("application/json").unwrap());

        let response = self.0.execute(request).await?;

        Ok(json::parse(&response.text().await?)?)
    }

    pub async fn get_nex_password(&mut self , pid: u32) -> Result<[u8; 16]>{
        let req = self.do_request(object!{
            "query": r"query($pid: Int!){
                userByPid(pid: $pid){
                    nexPassword
                }
            }",
            "variables": {
                "pid": pid
            }
        }).await?;

        let Some(val) = req.entries()
            .find(|v| v.0 == "data")
            .ok_or(SomethingHappened)?.1
            .entries()
            .find(|v| v.0 == "userByPid")
            .ok_or(SomethingHappened)?.1
            .entries()
            .find(|v| v.0 == "nexPassword")
            .ok_or(SomethingHappened)?.1
            .as_str() else {
            return Err(SomethingHappened);
        };

        Ok(val.as_bytes().try_into().map_err(|_| SomethingHappened)?)
    }

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
#[cfg(test)]
mod test{
    use crate::grpc::account::Client;

    #[tokio::test]
    async fn test(){
        dotenv::dotenv().ok();

        let mut client = Client::new().await.unwrap();

        let cli = client.get_nex_password(1699562916).await.unwrap();

        println!("{:?}", cli);
    }


}