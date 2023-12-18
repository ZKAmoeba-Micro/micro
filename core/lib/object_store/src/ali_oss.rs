//! Aliyun OSS-based [`ObjectStore`] implementation.

use std::{collections::HashMap, fmt, fs, future::Future, time::Duration};

use async_trait::async_trait;
use oss_rust_sdk::{async_object::*, oss::OSS};
use serde::{Deserialize, Serialize};

use crate::raw::{Bucket, ObjectStore, ObjectStoreError};

async fn retry<T, E, Fut, F>(max_retries: u16, mut f: F) -> Result<T, E>
where
    E: fmt::Display,
    Fut: Future<Output = Result<T, E>>,
    F: FnMut() -> Fut,
{
    let mut retries = 1;
    let mut backoff = 1;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                // tracing::warn!(%err, "Failed OSS request {retries}/{max_retries}, retrying.");
                if retries > max_retries {
                    return Err(err);
                }
                retries += 1;
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff *= 2;
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientConfig {
    pub access_key_id: String,
    pub access_key_secret: String,
    pub endpoint: String,
    pub bucket: String,
}

pub struct AliyunOssStorage<'a> {
    max_retries: u16,
    bucket_prefix: String,
    client: OSS<'a>,
}

impl fmt::Debug for AliyunOssStorage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AliyunOss")
            .field("bucket_prefix", &self.bucket_prefix)
            .field("max_retries", &self.max_retries)
            .finish()
    }
}

impl<'a> AliyunOssStorage<'a> {
    pub fn new(
        credential_file_path: String,
        bucket_prefix: String,
        max_retries: u16,
    ) -> AliyunOssStorage<'a> {
        let data =
            fs::read_to_string(credential_file_path).expect("failed open ali oss config file");

        let client_config: ClientConfig =
            serde_json::from_str(&data).expect("failed loading ali oss config file");

        let client = OSS::new(
            client_config.access_key_id,
            client_config.access_key_secret,
            client_config.endpoint,
            client_config.bucket,
        );

        Self {
            bucket_prefix,
            max_retries,
            client,
        }
    }

    fn filename(&self, bucket_prefix: &str, bucket: &str, filename: &str) -> String {
        format!("{bucket_prefix}/{bucket}/{filename}")
    }
}

impl From<oss_rust_sdk::errors::Error> for ObjectStoreError {
    fn from(err: oss_rust_sdk::errors::Error) -> Self {
        if err.to_string().contains("404 Not Found") {
            ObjectStoreError::KeyNotFound(err.into())
        } else {
            ObjectStoreError::Other(err.into())
        }
    }
}

#[async_trait]
impl<'a> ObjectStore for AliyunOssStorage<'a> {
    async fn get_raw(&self, bucket: Bucket, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let result = retry(self.max_retries, || {
            let extar_header: HashMap<&str, &str> = HashMap::new();
            self.client.get_object(
                self.filename(&self.bucket_prefix, bucket.as_str(), key),
                extar_header,
                None,
            )
        })
        .await?;
        Ok(result.to_vec())
    }

    async fn put_raw(
        &self,
        bucket: Bucket,
        key: &str,
        value: Vec<u8>,
    ) -> Result<(), ObjectStoreError> {
        retry(self.max_retries, || {
            let extar_header: HashMap<&str, &str> = HashMap::new();

            let filename = self.filename(&self.bucket_prefix, bucket.as_str(), key);

            self.client.put_object(&value, filename, extar_header, None)
        })
        .await?;
        Ok(())
    }

    async fn remove_raw(&self, bucket: Bucket, key: &str) -> Result<(), ObjectStoreError> {
        retry(self.max_retries, || {
            self.client
                .delete_object(self.filename(&self.bucket_prefix, bucket.as_str(), key))
        })
        .await?;
        Ok(())
    }

    fn storage_prefix_raw(&self, bucket: Bucket) -> String {
        format!("{}/{}", self.bucket_prefix, bucket)
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicU16, Ordering};

    use super::*;

    #[tokio::test]
    async fn test_retry_success_immediate() {
        let result = retry(2, || async { Ok::<_, &'static str>(42) }).await;
        assert_eq!(result, Ok(42));
    }

    #[tokio::test]
    async fn test_retry_failure_exhausted() {
        let result = retry(2, || async { Err::<i32, _>("oops") }).await;
        assert_eq!(result, Err("oops"));
    }

    async fn retry_success_after_n_retries(n: u16) -> Result<u32, String> {
        let retries = AtomicU16::new(0);
        let result = retry(n, || async {
            let retries = retries.fetch_add(1, Ordering::Relaxed);
            if retries + 1 == n {
                Ok(42)
            } else {
                Err("oops")
            }
        })
        .await;

        result.map_err(|_| "Retry failed".to_string())
    }

    #[tokio::test]
    async fn test_retry_success_after_retry() {
        let result = retry(2, || retry_success_after_n_retries(2)).await;
        assert_eq!(result, Ok(42));
    }

    fn new_storage() -> AliyunOssStorage<'static> {
        AliyunOssStorage::new("/home/qw/oss.json".into(), "artifacts".into(), 3)
    }

    #[tokio::test]
    async fn test_upload() {
        let oss = new_storage();
        let result = oss
            .put_raw(crate::Bucket::WitnessInput, "test", "test string".into())
            .await
            .unwrap();
        assert_eq!(result, ());
    }

    #[tokio::test]
    async fn test_get() {
        let oss = new_storage();
        let result = oss
            .get_raw(crate::Bucket::WitnessInput, "test")
            .await
            .unwrap();
        let value: Vec<u8> = "test string".into();
        assert_eq!(result, value);
    }

    #[tokio::test]
    async fn test_remove() {
        let oss = new_storage();
        let result = oss
            .remove_raw(crate::Bucket::WitnessInput, "test")
            .await
            .unwrap();
        assert_eq!(result, ());
    }
}
