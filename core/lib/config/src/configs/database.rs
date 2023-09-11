use serde::Deserialize;

use std::{env, str::FromStr, time::Duration};

/// Database configuration.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DBConfig {
    /// Path to the database data directory.
    pub path: String,
    /// Path to the database data directory that serves state cache.
    pub state_keeper_db_path: String,
    /// Path to merkle tree backup directory.
    pub merkle_tree_backup_path: String,
    /// Fast SSD path. Used as a RocksDB dir for the Merkle tree (*old* implementation)
    /// if the lightweight syncing is enabled.
    pub merkle_tree_fast_ssd_path: String,
    /// Fast SSD path. Used as a RocksDB dir for the Merkle tree (*new* implementation).
    // We cannot compute this path like
    //
    // ```
    // new_merkle_tree_ssd_path = merkle_tree_fast_ssd_path.join("new")
    // ```
    //
    // because (1) we need to maintain backward compatibility; (2) it looks dangerous
    // to place a RocksDB instance in a subdirectory of another RocksDB instance.
    pub new_merkle_tree_ssd_path: String,
    /// Throttle interval for the new tree implementation in milliseconds. This interval will be
    /// applied after each time the tree makes progress.
    pub new_merkle_tree_throttle_ms: u64,
    /// Number of backups to keep.
    pub backup_count: usize,
    /// Time interval between performing backups.
    pub backup_interval_ms: u64,
    /// Maximum number of blocks to be processed by the full tree at a time.
    pub max_block_batch: usize,
}

impl Default for DBConfig {
    fn default() -> Self {
        Self {
            path: "./db".to_owned(),
            state_keeper_db_path: "./db/state_keeper".to_owned(),
            merkle_tree_backup_path: "./db/backups".to_owned(),
            merkle_tree_fast_ssd_path: "./db/lightweight".to_owned(),
            new_merkle_tree_ssd_path: "./db/lightweight-new".to_owned(),
            new_merkle_tree_throttle_ms: 0,
            backup_count: 5,
            backup_interval_ms: 60_000,
            max_block_batch: 100,
        }
    }
}

impl DBConfig {
    pub fn from_env() -> Self {
        let mut config = DBConfig::default();
        if let Ok(path) = env::var("DATABASE_PATH") {
            config.path = path;
        }
        if let Ok(path) = env::var("DATABASE_STATE_KEEPER_DB_PATH") {
            config.state_keeper_db_path = path;
        }
        if let Ok(path) = env::var("DATABASE_MERKLE_TREE_BACKUP_PATH") {
            config.merkle_tree_backup_path = path;
        }
        if let Ok(path) = env::var("DATABASE_MERKLE_TREE_FAST_SSD_PATH") {
            config.merkle_tree_fast_ssd_path = path;
        }
        if let Ok(path) = env::var("DATABASE_NEW_MERKLE_TREE_SSD_PATH") {
            config.new_merkle_tree_ssd_path = path;
        }
        if let Some(interval) = Self::parse_env_var("DATABASE_NEW_MERKLE_TREE_THROTTLE_MS") {
            config.new_merkle_tree_throttle_ms = interval;
        }
        if let Some(count) = Self::parse_env_var("DATABASE_BACKUP_COUNT") {
            config.backup_count = count;
        }
        if let Some(interval) = Self::parse_env_var("DATABASE_BACKUP_INTERVAL_MS") {
            config.backup_interval_ms = interval;
        }
        if let Some(size) = Self::parse_env_var("DATABASE_MAX_BLOCK_BATCH") {
            config.max_block_batch = size;
        }
        config
    }

    fn parse_env_var<T: FromStr>(key: &str) -> Option<T> {
        let env_var = env::var(key).ok()?;
        env_var.parse().ok()
    }

    /// Path to the database data directory.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Path to the database data directory that serves state cache.
    pub fn state_keeper_db_path(&self) -> &str {
        &self.state_keeper_db_path
    }

    /// Path to the merkle tree backup directory.
    pub fn merkle_tree_backup_path(&self) -> &str {
        &self.merkle_tree_backup_path
    }

    pub fn merkle_tree_fast_ssd_path(&self) -> &str {
        &self.merkle_tree_fast_ssd_path
    }

    /// Throttle interval for the new Merkle tree implementation.
    pub fn new_merkle_tree_throttle_interval(&self) -> Duration {
        Duration::from_millis(self.new_merkle_tree_throttle_ms)
    }

    /// Number of backups to keep
    pub fn backup_count(&self) -> usize {
        self.backup_count
    }

    pub fn backup_interval(&self) -> Duration {
        Duration::from_millis(self.backup_interval_ms)
    }

    pub fn max_block_batch(&self) -> usize {
        self.max_block_batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configs::test_utils::set_env;

    #[test]
    fn from_env() {
        let config = r#"
DATABASE_PATH="./db"
DATABASE_STATE_KEEPER_DB_PATH="./db/state_keeper"
DATABASE_MERKLE_TREE_BACKUP_PATH="./db/backups"
DATABASE_MERKLE_TREE_FAST_SSD_PATH="./db/lightweight"
DATABASE_NEW_MERKLE_TREE_SSD_PATH="./db/lightweight-new"
DATABASE_NEW_MERKLE_TREE_THROTTLE_MS=0
DATABASE_BACKUP_COUNT=5
DATABASE_BACKUP_INTERVAL_MS=60000
DATABASE_MAX_BLOCK_BATCH=100
        "#;
        set_env(config);

        let actual = DBConfig::from_env();
        assert_eq!(actual, DBConfig::default());
    }

    /// Checks the correctness of the config helper methods.
    #[test]
    fn methods() {
        let config = DBConfig::default();

        assert_eq!(config.path(), &config.path);
        assert_eq!(config.state_keeper_db_path(), &config.state_keeper_db_path);
        assert_eq!(
            config.merkle_tree_backup_path(),
            &config.merkle_tree_backup_path
        );
        assert_eq!(
            config.merkle_tree_fast_ssd_path(),
            &config.merkle_tree_fast_ssd_path
        );
        assert_eq!(config.backup_count(), config.backup_count);
        assert_eq!(config.backup_interval().as_secs(), 60);
    }
}
