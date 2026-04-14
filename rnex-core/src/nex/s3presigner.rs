use aws_sdk_s3::presigning::PresigningConfig;
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use sha2::{Sha256, Digest};
use chrono::{Utc, Duration};
use serde_json::json;
use rnex_core::executables::common::RNEX_DATASTORE_S3_ENDPOINT;

pub struct S3Presigner {
    endpoint: String,
    bucket: String,
}

impl S3Presigner {
    pub async fn new(endpoint: &str, bucket: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            bucket,
        }
    }
    pub async fn generate_presigned_post(&self, key: &str) -> (String, Vec<(String, String)>) {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID").expect("Missing Access Key");
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").expect("Missing Secret Key");
        let region = "us-east-1"; // hardcoded because its the default region for most s3 clones
        let date_short = Utc::now().format("%Y%m%d").to_string();
        let date_full = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
        let expiration = (Utc::now() + Duration::minutes(15)).format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let credential = format!("{}/{}/{}/s3/aws4_request", access_key, date_short, region);

        let policy_json = json!({
            "expiration": expiration,
            "conditions": [
                {"bucket": self.bucket},
                ["starts-with", "$key", key],
                {"x-amz-credential": credential},
                {"x-amz-algorithm": "AWS4-HMAC-SHA256"},
                {"x-amz-date": date_full}
            ]
        });

        let policy_base64 = general_purpose::STANDARD.encode(policy_json.to_string());

        let signature = self.calculate_signature(&secret_key, &date_short, region, &policy_base64);

        let mut fields = vec![
            ("key".to_string(), key.to_string()),
            ("X-Amz-Algorithm".to_string(), "AWS4-HMAC-SHA256".to_string()),
            ("X-Amz-Credential".to_string(), credential),
            ("X-Amz-Date".to_string(), date_full),
            ("Policy".to_string(), policy_base64),
            ("X-Amz-Signature".to_string(), signature),
        ];

        let url = format!("https://{}/{}", *RNEX_DATASTORE_S3_ENDPOINT, self.bucket);
        (url, fields)
    }

    fn calculate_signature(&self, secret: &str, date: &str, region: &str, policy: &str) -> String {
        let k_date = self.hmac_sha256(format!("AWS4{}", secret).as_bytes(), date);
        let k_region = self.hmac_sha256(&k_date, region);
        let k_service = self.hmac_sha256(&k_region, "s3");
        let k_signing = self.hmac_sha256(&k_service, "aws4_request");

        hex::encode(self.hmac_sha256(&k_signing, policy))
    }

    fn hmac_sha256(&self, key: &[u8], data: &str) -> Vec<u8> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
}