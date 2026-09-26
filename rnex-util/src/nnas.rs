use std::env;

use hmac::{Hmac, KeyInit, Mac};
use serde::Deserialize;
use sha2::Sha256;
use thiserror::Error;

use crate::PID;

const DEFAULT_NNAS_URL: &str = "http://127.0.0.1:8080";
const INTERNAL_TIMESTAMP_HEADER: &str = "X-NNAS-Timestamp";

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
pub enum NnasError {
    #[error("NNAS rejected the NEX token")]
    InvalidToken,
    #[error("missing required configuration: {0}")]
    MissingConfiguration(&'static str),
    #[error("NNAS rejected the internal service credential")]
    InternalAccessDenied,
    #[error("NNAS has no available login data for PID {0}")]
    LoginDataUnavailable(PID),
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

pub async fn get_nex_login_data(pid: PID) -> Result<NexLoginData, NnasError> {
    let base_url = env::var("NNAS_URL").unwrap_or_else(|_| DEFAULT_NNAS_URL.to_owned());
    let internal_token = env::var("NNAS_INTERNAL_TOKEN")
        .map_err(|_| NnasError::MissingConfiguration("NNAS_INTERNAL_TOKEN"))?;
    let path = format!("/api/v2/nex/internal/login_data/{pid}");
    let url = format!("{}{path}", base_url.trim_end_matches('/'));
    let timestamp = chrono::Utc::now().timestamp();
    let authorization = internal_authorization(&internal_token, &path, timestamp)?;

    tokio::task::spawn_blocking(move || {
        let mut response = ureq::post(&url)
            .header("Authorization", &authorization)
            .header(INTERNAL_TIMESTAMP_HEADER, timestamp.to_string())
            .send_empty()
            .map_err(|error| match error {
                ureq::Error::StatusCode(401) => NnasError::InternalAccessDenied,
                ureq::Error::StatusCode(403 | 404) => NnasError::LoginDataUnavailable(pid),
                other => NnasError::Request(other.to_string()),
            })?;

        let data = response
            .body_mut()
            .read_json::<NexLoginData>()
            .map_err(|error| NnasError::InvalidResponse(error.to_string()))?;
        if data.rnex_pid() != pid {
            return Err(NnasError::InvalidResponse(
                "response PID did not match the requested PID".to_owned(),
            ));
        }
        Ok(data)
    })
    .await
    .map_err(|error| NnasError::Task(error.to_string()))?
}

fn internal_authorization(
    internal_token: &str,
    path: &str,
    timestamp: i64,
) -> Result<String, NnasError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(internal_token.as_bytes())
        .map_err(|error| NnasError::Request(error.to_string()))?;
    mac.update(format!("{timestamp}\nPOST\n{path}").as_bytes());
    Ok(format!(
        "NNAS-HMAC {}",
        hex::encode(mac.finalize().into_bytes())
    ))
}

#[cfg(test)]
mod tests {
    use super::internal_authorization;

    #[test]
    fn internal_authorization_matches_the_nnas_signature_format() {
        assert_eq!(
            internal_authorization(
                "shared secret",
                "/api/v2/nex/internal/login_data/1074",
                1_000
            )
            .unwrap(),
            "NNAS-HMAC 6d07e931b78f8d16fe5afe7be6404ef9cb41b214d733ff8422ff67cc996e26de"
        );
    }
}
