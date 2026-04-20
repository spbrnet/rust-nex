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

    pub fn generate_presigned_get(&self, key: &str) -> String {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID").expect("Missing Access Key");
        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").expect("Missing Secret Key");
        let region = "us-east-1";
        let date_short = Utc::now().format("%Y%m%d").to_string();
        let date_full = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();

        let credential_scope = format!("{}/{}/s3/aws4_request", date_short, region);

        let query_string = format!(
            "X-Amz-Algorithm=AWS4-HMAC-SHA256&\
             X-Amz-Credential={}%2F{}&\
             X-Amz-Date={}&\
             X-Amz-Expires=900&\
             X-Amz-SignedHeaders=host",
            access_key,
            urlencoding::encode(&credential_scope),
            date_full
        );

        let canonical_request = format!(
            "GET\n/{}/{}\n{}\nhost:{}\n\nhost\nUNSIGNED-PAYLOAD",
            self.bucket, key, query_string, *RNEX_DATASTORE_S3_ENDPOINT
        );

        let hashed_request = hex::encode(Sha256::digest(canonical_request.as_bytes()));

        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            date_full, credential_scope, hashed_request
        );

        let k_date = self.hmac_sha256(format!("AWS4{}", secret_key).as_bytes(), &date_short);
        let k_region = self.hmac_sha256(&k_date, region);
        let k_service = self.hmac_sha256(&k_region, "s3");
        let k_signing = self.hmac_sha256(&k_service, "aws4_request");
        let signature = hex::encode(self.hmac_sha256(&k_signing, &string_to_sign));

        format!(
            "https://{}/{}/{}?{}&X-Amz-Signature={}",
            *RNEX_DATASTORE_S3_ENDPOINT, self.bucket, key, query_string, signature
        )
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