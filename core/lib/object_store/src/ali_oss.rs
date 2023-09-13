//! Aliyun OSS-based [`ObjectStore`] implementation.

use oss_rust_sdk::prelude::*;
use serde::{Deserialize, Serialize};

use std::{collections::HashMap, fmt, fs, thread, time::Duration};

use crate::raw::{Bucket, ObjectStore, ObjectStoreError};

fn retry<T, E, F>(max_retries: u16, mut f: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let mut retries = 1;
    let mut backoff = 1;
    loop {
        match f() {
            Ok(result) => return Ok(result),
            Err(err) => {
                vlog::warn!("Failed oss request {retries}/{max_retries}, retrying.");
                if retries > max_retries {
                    return Err(err);
                }
                retries += 1;
                thread::sleep(Duration::from_secs(backoff));
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
        ObjectStoreError::Other(err.into())
    }
}

impl<'a> ObjectStore for AliyunOssStorage<'a> {
    fn get_raw(&self, bucket: Bucket, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let result = retry(self.max_retries, || {
            let extar_header: HashMap<&str, &str> = HashMap::new();
            // let mut oss_sub_resource = HashMap::new();
            // oss_sub_resource.insert("acl", None);
            // oss_sub_resource.insert("response-content-type", Some("ContentType"));
            self.client.get_object(
                self.filename(&self.bucket_prefix, bucket.as_str(), key),
                extar_header,
                None,
            )
        })?;
        Ok(result)
    }

    fn put_raw(&self, bucket: Bucket, key: &str, value: Vec<u8>) -> Result<(), ObjectStoreError> {
        println!("oss {:?}", self.client);

        let result = retry(self.max_retries, || {
            let extar_header: HashMap<&str, &str> = HashMap::new();

            let filename = self.filename(&self.bucket_prefix, bucket.as_str(), key);

            self.client
                .put_object_from_buffer(&value, filename, extar_header, None)
        })?;
        Ok(result)
    }

    fn remove_raw(&self, bucket: Bucket, key: &str) -> Result<(), ObjectStoreError> {
        let result = retry(self.max_retries, || {
            self.client
                .delete_object(self.filename(&self.bucket_prefix, bucket.as_str(), key))
        })?;
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicU16, Ordering};

    use super::*;

    #[test]
    fn test_retry_success_immediate() {
        let result = retry(2, || Ok::<_, ()>(42));
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn test_retry_failure_exhausted() {
        let result = retry(2, || Err::<i32, _>(()));
        assert_eq!(result, Err(()));
    }

    fn retry_success_after_n_retries(n: u16) -> Result<u32, String> {
        let retries = AtomicU16::new(0);
        let result = retry(n, || {
            let retries = retries.fetch_add(1, Ordering::Relaxed);
            if retries + 1 == n {
                Ok(42)
            } else {
                Err(())
            }
        });

        result.map_err(|_| "Retry failed".to_string())
    }

    #[test]
    fn test_retry_success_after_retry() {
        let result = retry(2, || retry_success_after_n_retries(2));
        assert_eq!(result, Ok(42));
    }

    fn new_storage() -> AliyunOssStorage<'static> {
        AliyunOssStorage::new("".into(), "artifacts".into(), 3)
    }

    #[test]
    fn test_upload() {
        let oss = new_storage();
        let result = oss
            .put_raw(crate::Bucket::WitnessInput, "test2", "test string".into())
            .unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_get() {
        let oss = new_storage();
        let result = oss.get_raw(crate::Bucket::WitnessInput, "test").unwrap();
        let value: Vec<u8> = "test string".into();
        assert_eq!(result, value);
    }

    #[test]
    fn test_remove() {
        let oss = new_storage();
        let result = oss.remove_raw(crate::Bucket::WitnessInput, "test").unwrap();
        assert_eq!(result, ());
    }
}
