use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::Write;

use crate::{Bucket, ObjectStore, ObjectStoreError};

impl From<reqwest::Error> for ObjectStoreError {
    fn from(err: reqwest::Error) -> Self {
        ObjectStoreError::Other(err.into())
    }
}

#[derive(Debug)]
pub struct HttpBackedObjectStore {
    base_dir: String,
    base_url: String,
}

impl HttpBackedObjectStore {
    pub fn new(base_dir: String, base_url: String) -> Self {
        for bucket in &[
            Bucket::ProverJobs,
            Bucket::WitnessInput,
            Bucket::LeafAggregationWitnessJobs,
            Bucket::NodeAggregationWitnessJobs,
            Bucket::SchedulerWitnessJobs,
        ] {
            fs::create_dir_all(format!("{}/{}", base_dir, bucket)).expect("failed creating bucket");
        }
        HttpBackedObjectStore { base_dir, base_url }
    }

    fn filename(&self, bucket: Bucket, key: &str) -> String {
        format!("{}/{}/{}", self.base_dir, bucket, key)
    }

    fn http_filename(&self, bucket: Bucket, key: &str) -> String {
        format!("{}/{}/{}", self.base_url, bucket, key)
    }
}

impl ObjectStore for HttpBackedObjectStore {
    fn get_raw(&self, bucket: Bucket, key: &str) -> Result<Vec<u8>, ObjectStoreError> {
        let resp = reqwest::blocking::get(self.http_filename(bucket, key))?.bytes()?;
        Ok(resp.to_vec())
    }

    // TODO need http
    fn put_raw(&self, bucket: Bucket, key: &str, value: Vec<u8>) -> Result<(), ObjectStoreError> {
        let filename = self.filename(bucket, key);
        let mut file = File::create(filename)?;
        file.write_all(&value)?;
        Ok(())
    }

    // TODO need http
    fn remove_raw(&self, bucket: Bucket, key: &str) -> Result<(), ObjectStoreError> {
        let filename = self.filename(bucket, key);
        fs::remove_file(filename)?;
        Ok(())
    }
}
