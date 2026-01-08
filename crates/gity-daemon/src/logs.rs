use bincode::Options as BincodeOptions;
use gity_ipc::{bounded_bincode, LogEntry};
use serde::{Deserialize, Serialize};
use sled::Tree;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use tracing::warn;

#[derive(Clone)]
pub struct LogBook {
    inner: Arc<Mutex<HashMap<PathBuf, VecDeque<LogEntryRecord>>>>,
    capacity: usize,
    tree: Option<Tree>,
}

#[derive(Clone, Debug)]
pub struct LogEntryRecord {
    pub timestamp: SystemTime,
    pub message: String,
}

impl LogBook {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            tree: None,
        }
    }

    pub fn with_persistence(capacity: usize, tree: Tree) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            tree: Some(tree),
        }
    }

    pub fn record(&self, repo_path: &Path, message: impl Into<String>) -> LogEntry {
        let entry = LogEntryRecord {
            timestamp: SystemTime::now(),
            message: message.into(),
        };
        {
            let mut guard = self.inner.lock().expect("log book poisoned");
            let buf = guard.entry(repo_path.to_path_buf()).or_default();
            buf.push_back(entry.clone());
            if buf.len() > self.capacity {
                buf.pop_front();
            }
        }
        if let Some(tree) = &self.tree {
            let record = PersistedLogEntry::from_entry(repo_path, &entry);
            if let Ok(bytes) = bounded_bincode().serialize(&record) {
                let key = log_key(repo_path, entry.timestamp);
                if let Err(err) = tree.insert(key, bytes) {
                    warn!("failed to persist log entry: {}", err);
                }
            }
        }
        LogEntry {
            repo_path: repo_path.to_path_buf(),
            message: entry.message.clone(),
            timestamp: entry.timestamp,
        }
    }

    pub fn recent(&self, repo_path: &Path, limit: usize) -> Vec<LogEntry> {
        let guard = self.inner.lock().expect("log book poisoned");
        guard
            .get(repo_path)
            .map(|buf| {
                buf.iter()
                    .rev()
                    .take(limit)
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(|record| LogEntry {
                        repo_path: repo_path.to_path_buf(),
                        message: record.message,
                        timestamp: record.timestamp,
                    })
                    .collect()
            })
            .unwrap_or_else(|| {
                if let Some(tree) = &self.tree {
                    load_from_tree(tree, repo_path, limit)
                } else {
                    Vec::new()
                }
            })
    }
}

fn log_key(repo_path: &Path, timestamp: SystemTime) -> Vec<u8> {
    let mut key = repo_path.to_string_lossy().as_bytes().to_vec();
    key.push(0);
    let ts = timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    key.extend_from_slice(&ts.to_be_bytes());
    key
}

fn load_from_tree(tree: &Tree, repo_path: &Path, limit: usize) -> Vec<LogEntry> {
    let prefix = {
        let mut p = repo_path.to_string_lossy().as_bytes().to_vec();
        p.push(0);
        p
    };
    // Enforce reasonable limits to prevent memory exhaustion
    let max_entries = limit.min(10_000);
    let entries: Vec<LogEntry> = tree
        .range(prefix.clone()..)
        .filter(|res| match res {
            Ok((key, _)) => key.starts_with(&prefix),
            Err(_) => false,
        })
        .rev()
        .take(max_entries)
        .filter_map(|result| {
            let (_, value) = result.ok()?;
            // Skip oversized entries
            if value.len() > MAX_LOG_ENTRY_SIZE {
                warn!("skipping oversized log entry: {} bytes", value.len());
                return None;
            }
            let record = bounded_bincode()
                .deserialize::<PersistedLogEntry>(&value)
                .ok()?;
            Some(LogEntry {
                repo_path: record.repo_path.clone(),
                message: record.message,
                timestamp: record.timestamp,
            })
        })
        .collect();
    entries
}

/// Maximum allowed log entry size (1MB)
const MAX_LOG_ENTRY_SIZE: usize = 1_048_576;

#[derive(Debug, Serialize, Deserialize)]
struct PersistedLogEntry {
    repo_path: PathBuf,
    message: String,
    timestamp: SystemTime,
}

impl PersistedLogEntry {
    fn from_entry(repo_path: &Path, entry: &LogEntryRecord) -> Self {
        Self {
            repo_path: repo_path.to_path_buf(),
            message: entry.message.clone(),
            timestamp: entry.timestamp,
        }
    }
}
