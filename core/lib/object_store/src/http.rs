use async_trait::async_trait;
use reqwest::Client;

use std::{
    fmt::{self, Debug},
    future::Future,
    time::Duration,
};

use crate::raw::{Bucket, ObjectStore, ObjectStoreError};

impl From<reqwest::Error> for ObjectStoreError {
    fn from(err: reqwest::Error) -> Self {
        match err.status() {
            Some(reqwest::StatusCode::NOT_FOUND) => ObjectStoreError::KeyNotFound(err.into()),
            _ => ObjectStoreError::Other(err.into()),
        }
    }
}

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
                tracing::warn!(%err, "Failed Http request {retries}/{max_retries}, retrying.");
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

#[derive(Debug)]
pub(crate) struct HttpBackedObjectStore {
    base_url: String,
    max_retries: u16,
}

impl HttpBackedObjectStore {
    pub fn new(base_url: String, max_retries: u16) -> Self {
        HttpBackedObjectStore {
            base_url,
            max_retries,
        }
    }

    fn path(&self, bucket: Bucket, key: &str) -> String {
        format!("{}/{bucket}/{key}", self.base_url)
    }
}

#[async_trait]
impl ObjectStore for HttpBackedObjectStore {
    async fn get_raw(&self, bucket: Bucket, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let resp = retry(self.max_retries, || reqwest::get(self.path(bucket, key))).await?;
        let blob = resp.bytes().await?;
        Ok(blob.to_vec())
    }

    async fn put_raw(
        &self,
        bucket: Bucket,
        key: &str,
        value: Vec<u8>,
    ) -> Result<(), ObjectStoreError> {
        retry(self.max_retries, || {
            Client::new()
                .put(self.path(bucket, key))
                .body(value.clone())
                .send()
        })
        .await?;

        Ok(())
    }

    async fn remove_raw(&self, bucket: Bucket, key: &str) -> Result<(), ObjectStoreError> {
        retry(self.max_retries, || {
            Client::new().delete(self.path(bucket, key)).send()
        })
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_get() {
        let base_url = "http://127.0.0.1:8000".to_string();
        let object_store = HttpBackedObjectStore::new(base_url, 3);
        let expected = vec![9, 0, 8, 9, 0, 7];
        let result = object_store
            .put_raw(Bucket::ProverJobs, "test-key.bin", expected.clone())
            .await;
        assert!(result.is_ok(), "result must be OK");
        let bytes = object_store
            .get_raw(Bucket::ProverJobs, "test-key.bin")
            .await
            .unwrap();
        assert_eq!(expected, bytes, "expected didn't match");
    }

    #[tokio::test]
    async fn test_put() {
        let base_url = "http://127.0.0.1:8000".to_string();
        let object_store = HttpBackedObjectStore::new(base_url, 3);
        let bytes = vec![9, 0, 8, 9, 0, 7];
        let result = object_store
            .put_raw(Bucket::ProverJobs, "test-key.bin", bytes)
            .await;
        assert!(result.is_ok(), "result must be OK");
    }

    #[tokio::test]
    async fn test_remove() {
        let base_url = "http://127.0.0.1:8000".to_string();
        let object_store = HttpBackedObjectStore::new(base_url, 3);
        let result = object_store
            .put_raw(Bucket::ProverJobs, "test-key.bin", vec![0, 1])
            .await;
        assert!(result.is_ok(), "result must be OK");
        let result = object_store
            .remove_raw(Bucket::ProverJobs, "test-key.bin")
            .await;
        assert!(result.is_ok(), "result must be OK");
    }
}
