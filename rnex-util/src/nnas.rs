use std::env;

use serde::Deserialize;
use thiserror::Error;

use crate::PID;

const DEFAULT_NNAS_URL: &str = "http://127.0.0.1:8080";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct NexTokenAccount {
    pub pid: i32,
    pub account_level: i32,
    pub mii_name: String,
    pub username: String,
    pub nex_password: String,
}

impl NexTokenAccount {
    pub fn rnex_pid(&self) -> PID {
        PID::from(self.pid)
    }
}

#[derive(Debug, Error)]
pub enum NnasError {
    #[error("NNAS rejected the NEX token")]
    InvalidToken,
    #[error("failed to communicate with NNAS: {0}")]
    Request(String),
    #[error("NNAS returned an invalid token response: {0}")]
    InvalidResponse(String),
    #[error("NNAS validation task failed: {0}")]
    Task(String),
}

pub async fn validate_nex_token(token: &str) -> Result<NexTokenAccount, NnasError> {
    let base_url = env::var("NNAS_URL").unwrap_or_else(|_| DEFAULT_NNAS_URL.to_owned());
    let url = format!(
        "{}/api/v2/nex/validate_token",
        base_url.trim_end_matches('/')
    );
    let authorization = format!("Bearer {token}");

    tokio::task::spawn_blocking(move || {
        let mut response = ureq::post(&url)
            .header("Authorization", &authorization)
            .send_empty()
            .map_err(|error| match error {
                ureq::Error::StatusCode(401) => NnasError::InvalidToken,
                other => NnasError::Request(other.to_string()),
            })?;

        response
            .body_mut()
            .read_json::<NexTokenAccount>()
            .map_err(|error| NnasError::InvalidResponse(error.to_string()))
    })
    .await
    .map_err(|error| NnasError::Task(error.to_string()))?
}
