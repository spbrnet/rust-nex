use std::env;

use serde::Deserialize;
use thiserror::Error;

use crate::PID;

const DEFAULT_ACCOUNT_URL: &str = "http://127.0.0.1:8080";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct NexTokenAccount {
    pub pid: i32,
    pub account_level: i32,
    pub mii_name: String,
    pub username: String,
    pub nex_password: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct NexLoginData {
    pub pid: i32,
    pub username: String,
    pub nex_password: String,
}

impl NexTokenAccount {
    pub fn rnex_pid(&self) -> PID {
        PID::from(self.pid)
    }
}

impl NexLoginData {
    pub fn rnex_pid(&self) -> PID {
        PID::from(self.pid)
    }
}

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("MystikoNX rejected the NEX token")]
    InvalidToken,
    #[error("missing required configuration: {0}")]
    MissingConfiguration(&'static str),
    #[error("MystikoNX rejected the internal service credential")]
    InternalAccessDenied,
    #[error("MystikoNX has no available login data for PID {0}")]
    LoginDataUnavailable(PID),
    #[error("failed to communicate with MystikoNX: {0}")]
    Request(String),
    #[error("MystikoNX returned an invalid response: {0}")]
    InvalidResponse(String),
    #[error("MystikoNX validation task failed: {0}")]
    Task(String),
}

fn account_config() -> Result<(String, String), AccountError> {
    let base_url =
        env::var("MYSTIKONX_ACCOUNT_URL").unwrap_or_else(|_| DEFAULT_ACCOUNT_URL.to_owned());
    let token = env::var("MYSTIKONX_INTERNAL_TOKEN")
        .map_err(|_| AccountError::MissingConfiguration("MYSTIKONX_INTERNAL_TOKEN"))?;
    Ok((base_url, token))
}

pub async fn validate_nex_token(token: &str) -> Result<NexTokenAccount, AccountError> {
    let (base_url, internal_token) = account_config()?;
    let url = format!(
        "{}/v1/internal/nex/validate",
        base_url.trim_end_matches('/')
    );
    let body = serde_json::json!({"token": token});
    let permit = crate::http::REQUEST_SLOTS
        .acquire()
        .await
        .map_err(|e| AccountError::Task(e.to_string()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut response = crate::http::agent()
            .post(&url)
            .header("X-Mystiko-Internal-Token", &internal_token)
            .send_json(&body)
            .map_err(|error| match error {
                ureq::Error::StatusCode(401 | 403) => AccountError::InvalidToken,
                other => AccountError::Request(other.to_string()),
            })?;
        response
            .body_mut()
            .read_json::<NexTokenAccount>()
            .map_err(|error| AccountError::InvalidResponse(error.to_string()))
    })
    .await
    .map_err(|error| AccountError::Task(error.to_string()))?
}

pub async fn get_nex_login_data(pid: PID) -> Result<NexLoginData, AccountError> {
    let (base_url, internal_token) = account_config()?;
    let url = format!(
        "{}/v1/internal/nex/login/{pid}",
        base_url.trim_end_matches('/')
    );
    let permit = crate::http::REQUEST_SLOTS
        .acquire()
        .await
        .map_err(|e| AccountError::Task(e.to_string()))?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let mut response = crate::http::agent()
            .get(&url)
            .header("X-Mystiko-Internal-Token", &internal_token)
            .call()
            .map_err(|error| match error {
                ureq::Error::StatusCode(401) => AccountError::InternalAccessDenied,
                ureq::Error::StatusCode(403 | 404) => AccountError::LoginDataUnavailable(pid),
                other => AccountError::Request(other.to_string()),
            })?;
        let data = response
            .body_mut()
            .read_json::<NexLoginData>()
            .map_err(|error| AccountError::InvalidResponse(error.to_string()))?;
        if data.rnex_pid() != pid {
            return Err(AccountError::InvalidResponse(
                "response PID did not match the requested PID".to_owned(),
            ));
        }
        Ok(data)
    })
    .await
    .map_err(|error| AccountError::Task(error.to_string()))?
}
