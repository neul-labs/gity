use gitz_ipc::RepoStatus;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::RwLock,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

pub type StorageResult<T> = Result<T, StorageError>;

/// Errors surfaced by storage implementations.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum StorageError {
    #[error("repository not registered: {0}")]
    NotFound(String),
    #[error("internal locking error")]
    Poisoned,
    #[error("storage backend error: {0}")]
    Backend(String),
}

/// Metadata tracked for every registered repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMetadata {
    pub repo_path: PathBuf,
    pub registered_at: SystemTime,
    pub last_event: Option<SystemTime>,
    pub status: RepoStatus,
    pub pending_jobs: usize,
    pub dirty_paths: Vec<PathBuf>,
    pub generation: u64,
    pub needs_reconciliation: Option<bool>,
    pub last_watcher_token: Option<u64>,
}

impl RepoMetadata {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            registered_at: SystemTime::now(),
            last_event: None,
            status: RepoStatus::Idle,
            pending_jobs: 0,
            dirty_paths: Vec::new(),
            generation: 0,
            needs_reconciliation: Some(false),
            last_watcher_token: None,
        }
    }
}

/// Primary abstraction for storing repository metadata.
pub trait MetadataStore: Send + Sync + 'static {
    fn register_repo(&self, repo_path: PathBuf) -> StorageResult<RepoMetadata>;
    fn unregister_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>>;
    fn get_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>>;
    fn list_repos(&self) -> StorageResult<Vec<RepoMetadata>>;
    fn update_repo_status(&self, repo_path: &Path, status: RepoStatus) -> StorageResult<()>;
    fn increment_jobs(&self, repo_path: &Path, delta: isize) -> StorageResult<RepoMetadata>;
    fn record_event(&self, repo_path: &Path, when: SystemTime) -> StorageResult<()>;
    fn mark_dirty_path(&self, repo_path: &Path, path: PathBuf) -> StorageResult<()>;
    fn drain_dirty_paths(&self, repo_path: &Path) -> StorageResult<Vec<PathBuf>>;
    fn dirty_path_count(&self, repo_path: &Path) -> StorageResult<usize>;
    fn current_generation(&self, repo_path: &Path) -> StorageResult<u64>;
    fn bump_generation(&self, repo_path: &Path) -> StorageResult<u64>;
    fn set_needs_reconciliation(&self, repo_path: &Path, needs: bool) -> StorageResult<()>;
    fn set_watcher_token(&self, repo_path: &Path, token: u64) -> StorageResult<()>;
}

/// In-memory `MetadataStore` used in tests and the current bootstrap binary.
pub struct InMemoryMetadataStore {
    inner: RwLock<HashMap<PathBuf, RepoMetadata>>,
}

impl InMemoryMetadataStore {
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryMetadataStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MetadataStore for InMemoryMetadataStore {
    fn register_repo(&self, repo_path: PathBuf) -> StorageResult<RepoMetadata> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .entry(repo_path.clone())
            .or_insert_with(|| RepoMetadata::new(repo_path));
        Ok(entry.clone())
    }

    fn unregister_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        Ok(guard.remove(repo_path))
    }

    fn get_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>> {
        let guard = self.inner.read().map_err(|_| StorageError::Poisoned)?;
        Ok(guard.get(repo_path).cloned())
    }

    fn list_repos(&self) -> StorageResult<Vec<RepoMetadata>> {
        let guard = self.inner.read().map_err(|_| StorageError::Poisoned)?;
        Ok(guard.values().cloned().collect())
    }

    fn update_repo_status(&self, repo_path: &Path, status: RepoStatus) -> StorageResult<()> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        entry.status = status;
        Ok(())
    }

    fn increment_jobs(&self, repo_path: &Path, delta: isize) -> StorageResult<RepoMetadata> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        if delta >= 0 {
            entry.pending_jobs = entry.pending_jobs.saturating_add(delta as usize);
        } else {
            entry.pending_jobs = entry.pending_jobs.saturating_sub(delta.unsigned_abs());
        }
        entry.status = job_status_for(entry.pending_jobs);
        Ok(entry.clone())
    }

    fn record_event(&self, repo_path: &Path, when: SystemTime) -> StorageResult<()> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        entry.last_event = Some(when);
        Ok(())
    }

    fn current_generation(&self, repo_path: &Path) -> StorageResult<u64> {
        let guard = self.inner.read().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        Ok(entry.generation)
    }

    fn bump_generation(&self, repo_path: &Path) -> StorageResult<u64> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        entry.generation = entry.generation.saturating_add(1);
        Ok(entry.generation)
    }

    fn mark_dirty_path(&self, repo_path: &Path, path: PathBuf) -> StorageResult<()> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        if !entry.dirty_paths.contains(&path) {
            entry.dirty_paths.push(path);
        }
        Ok(())
    }

    fn drain_dirty_paths(&self, repo_path: &Path) -> StorageResult<Vec<PathBuf>> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        Ok(std::mem::take(&mut entry.dirty_paths))
    }

    fn dirty_path_count(&self, repo_path: &Path) -> StorageResult<usize> {
        let guard = self.inner.read().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        Ok(entry.dirty_paths.len())
    }

    fn set_needs_reconciliation(&self, repo_path: &Path, needs: bool) -> StorageResult<()> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        entry.needs_reconciliation = Some(needs);
        Ok(())
    }

    fn set_watcher_token(&self, repo_path: &Path, token: u64) -> StorageResult<()> {
        let mut guard = self.inner.write().map_err(|_| StorageError::Poisoned)?;
        let entry = guard
            .get_mut(repo_path)
            .ok_or_else(|| StorageError::NotFound(repo_path.display().to_string()))?;
        entry.last_watcher_token = Some(token);
        Ok(())
    }
}

/// Persistent sled-backed metadata store.
#[derive(Clone)]
pub struct SledMetadataStore {
    tree: sled::Tree,
}

impl SledMetadataStore {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let db = sled::open(path).map_err(map_sled_err)?;
        let tree = db.open_tree("repos").map_err(map_sled_err)?;
        Ok(Self { tree })
    }

    fn load_repo(&self, repo_path: &Path) -> StorageResult<RepoMetadata> {
        let key = repo_key(repo_path);
        let Some(bytes) = self.tree.get(&key).map_err(map_sled_err)? else {
            return Err(StorageError::NotFound(repo_path.display().to_string()));
        };
        deserialize_record(bytes.as_ref())
    }

    fn write_repo(&self, metadata: &RepoMetadata) -> StorageResult<()> {
        let key = repo_key(&metadata.repo_path);
        let record = RepoRecord::from(metadata);
        let bytes =
            bincode::serialize(&record).map_err(|err| StorageError::Backend(err.to_string()))?;
        self.tree.insert(key, bytes).map_err(map_sled_err)?;
        Ok(())
    }

    fn update_repo<F>(&self, repo_path: &Path, mutator: F) -> StorageResult<RepoMetadata>
    where
        F: FnOnce(&mut RepoMetadata),
    {
        let mut current = self.load_repo(repo_path)?;
        mutator(&mut current);
        self.write_repo(&current)?;
        Ok(current)
    }
}

impl MetadataStore for SledMetadataStore {
    fn register_repo(&self, repo_path: PathBuf) -> StorageResult<RepoMetadata> {
        let key = repo_key(&repo_path);
        if let Some(existing) = self.tree.get(&key).map_err(map_sled_err)? {
            return deserialize_record(existing.as_ref());
        }
        let metadata = RepoMetadata::new(repo_path);
        self.write_repo(&metadata)?;
        Ok(metadata)
    }

    fn unregister_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>> {
        let key = repo_key(repo_path);
        let result = self.tree.remove(&key).map_err(map_sled_err)?;
        Ok(match result {
            Some(bytes) => Some(deserialize_record(bytes.as_ref())?),
            None => None,
        })
    }

    fn get_repo(&self, repo_path: &Path) -> StorageResult<Option<RepoMetadata>> {
        let key = repo_key(repo_path);
        match self.tree.get(&key).map_err(map_sled_err)? {
            Some(bytes) => Ok(Some(deserialize_record(bytes.as_ref())?)),
            None => Ok(None),
        }
    }

    fn list_repos(&self) -> StorageResult<Vec<RepoMetadata>> {
        let mut repos = Vec::new();
        for entry in self.tree.iter() {
            let (_, value) = entry.map_err(map_sled_err)?;
            repos.push(deserialize_record(value.as_ref())?);
        }
        Ok(repos)
    }

    fn update_repo_status(&self, repo_path: &Path, status: RepoStatus) -> StorageResult<()> {
        self.update_repo(repo_path, |meta| meta.status = status)?;
        Ok(())
    }

    fn increment_jobs(&self, repo_path: &Path, delta: isize) -> StorageResult<RepoMetadata> {
        self.update_repo(repo_path, |meta| {
            if delta >= 0 {
                meta.pending_jobs = meta.pending_jobs.saturating_add(delta as usize);
            } else {
                meta.pending_jobs = meta.pending_jobs.saturating_sub(delta.unsigned_abs());
            }
            meta.status = job_status_for(meta.pending_jobs);
        })
    }

    fn record_event(&self, repo_path: &Path, when: SystemTime) -> StorageResult<()> {
        self.update_repo(repo_path, |meta| meta.last_event = Some(when))?;
        Ok(())
    }

    fn mark_dirty_path(&self, repo_path: &Path, path: PathBuf) -> StorageResult<()> {
        self.update_repo(repo_path, |meta| {
            if !meta.dirty_paths.contains(&path) {
                meta.dirty_paths.push(path);
            }
        })?;
        Ok(())
    }

    fn drain_dirty_paths(&self, repo_path: &Path) -> StorageResult<Vec<PathBuf>> {
        let mut removed = Vec::new();
        self.update_repo(repo_path, |meta| {
            removed = std::mem::take(&mut meta.dirty_paths);
        })?;
        Ok(removed)
    }

    fn current_generation(&self, repo_path: &Path) -> StorageResult<u64> {
        self.load_repo(repo_path).map(|meta| meta.generation)
    }

    fn bump_generation(&self, repo_path: &Path) -> StorageResult<u64> {
        let mut generation = 0;
        self.update_repo(repo_path, |meta| {
            meta.generation = meta.generation.saturating_add(1);
            generation = meta.generation;
        })?;
        Ok(generation)
    }

    fn dirty_path_count(&self, repo_path: &Path) -> StorageResult<usize> {
        self.load_repo(repo_path).map(|meta| meta.dirty_paths.len())
    }

    fn set_needs_reconciliation(&self, repo_path: &Path, needs: bool) -> StorageResult<()> {
        self.update_repo(repo_path, |meta| {
            meta.needs_reconciliation = Some(needs);
        })?;
        Ok(())
    }

    fn set_watcher_token(&self, repo_path: &Path, token: u64) -> StorageResult<()> {
        self.update_repo(repo_path, |meta| {
            meta.last_watcher_token = Some(token);
        })?;
        Ok(())
    }
}

fn repo_key(path: &Path) -> Vec<u8> {
    path.to_string_lossy().as_bytes().to_vec()
}

fn job_status_for(pending: usize) -> RepoStatus {
    if pending > 0 {
        RepoStatus::Busy
    } else {
        RepoStatus::Idle
    }
}

#[derive(Serialize, Deserialize)]
struct RepoRecord {
    repo_path: PathBuf,
    registered_at: u64,
    last_event: Option<u64>,
    status: RepoStatus,
    pending_jobs: usize,
    dirty_paths: Vec<PathBuf>,
    generation: u64,
    #[serde(default)]
    needs_reconciliation: Option<bool>,
    #[serde(default)]
    last_watcher_token: Option<u64>,
}

impl From<&RepoMetadata> for RepoRecord {
    fn from(value: &RepoMetadata) -> Self {
        Self {
            repo_path: value.repo_path.clone(),
            registered_at: encode_time(value.registered_at),
            last_event: value.last_event.map(encode_time),
            status: value.status.clone(),
            pending_jobs: value.pending_jobs,
            dirty_paths: value.dirty_paths.clone(),
            generation: value.generation,
            needs_reconciliation: value.needs_reconciliation,
            last_watcher_token: value.last_watcher_token,
        }
    }
}

impl From<RepoRecord> for RepoMetadata {
    fn from(value: RepoRecord) -> Self {
        Self {
            repo_path: value.repo_path,
            registered_at: decode_time(value.registered_at),
            last_event: value.last_event.map(decode_time),
            status: value.status,
            pending_jobs: value.pending_jobs,
            dirty_paths: value.dirty_paths,
            generation: value.generation,
            needs_reconciliation: value.needs_reconciliation,
            last_watcher_token: value.last_watcher_token,
        }
    }
}

fn encode_time(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn decode_time(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

fn deserialize_record(bytes: &[u8]) -> StorageResult<RepoMetadata> {
    let record: RepoRecord =
        bincode::deserialize(bytes).map_err(|err| StorageError::Backend(err.to_string()))?;
    Ok(record.into())
}

fn map_sled_err<E: std::fmt::Display>(err: E) -> StorageError {
    StorageError::Backend(err.to_string())
}

/// Helper that aligns all persisted artifacts (sled, future caches) under a
/// single directory.
#[derive(Debug, Clone)]
pub struct StorageContext {
    metadata_path: PathBuf,
    log_path: PathBuf,
}

impl StorageContext {
    /// Creates the metadata directory (if missing) beneath `data_root`.
    pub fn new(data_root: impl AsRef<Path>) -> StorageResult<Self> {
        let metadata_path = data_root.as_ref().join("sled");
        let log_path = data_root.as_ref().join("logs");
        fs::create_dir_all(&metadata_path).map_err(map_sled_err)?;
        fs::create_dir_all(&log_path).map_err(map_sled_err)?;
        Ok(Self {
            metadata_path,
            log_path,
        })
    }

    /// Returns a sled-backed metadata store rooted at this context's path.
    pub fn metadata_store(&self) -> StorageResult<SledMetadataStore> {
        SledMetadataStore::open(&self.metadata_path)
    }

    pub fn log_tree(&self) -> StorageResult<sled::Tree> {
        let db = sled::open(&self.log_path).map_err(map_sled_err)?;
        db.open_tree("logs").map_err(map_sled_err)
    }

    pub fn metadata_path(&self) -> &Path {
        &self.metadata_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    #[test]
    fn register_and_list_repositories() {
        let store = InMemoryMetadataStore::new();
        store
            .register_repo(PathBuf::from("/tmp/demo"))
            .expect("register");
        let repos = store.list_repos().expect("list");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].repo_path, PathBuf::from("/tmp/demo"));
    }

    #[test]
    fn unregister_repository() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        let removed = store.unregister_repo(&path).unwrap();
        assert!(removed.is_some());
        assert!(store.unregister_repo(&path).unwrap().is_none());
    }

    #[test]
    fn job_counters_do_not_underflow() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        store.increment_jobs(&path, 5).unwrap();
        let snapshot = store.increment_jobs(&path, -10).unwrap();
        assert_eq!(snapshot.pending_jobs, 0);
    }

    #[test]
    fn in_memory_job_status_tracks_pending_jobs() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        let initial = store.register_repo(path.clone()).unwrap();
        assert_eq!(initial.status, RepoStatus::Idle);
        let snapshot = store.increment_jobs(&path, 1).unwrap();
        assert_eq!(snapshot.status, RepoStatus::Busy);
        let snapshot = store.increment_jobs(&path, -1).unwrap();
        assert_eq!(snapshot.status, RepoStatus::Idle);
    }

    #[test]
    fn record_last_event() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        store.record_event(&path, UNIX_EPOCH).unwrap();
        let repos = store.list_repos().unwrap();
        assert_eq!(repos[0].last_event, Some(UNIX_EPOCH));
    }

    #[test]
    fn sled_store_persists_between_instances() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("db");
        {
            let store = SledMetadataStore::open(&db_path).unwrap();
            store
                .register_repo(PathBuf::from("/tmp/demo"))
                .expect("register");
        }
        {
            let store = SledMetadataStore::open(&db_path).unwrap();
            let repos = store.list_repos().unwrap();
            assert_eq!(repos.len(), 1);
            assert_eq!(repos[0].repo_path, PathBuf::from("/tmp/demo"));
        }
    }

    #[test]
    fn sled_store_updates_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let store = SledMetadataStore::open(dir.path()).unwrap();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        store.increment_jobs(&path, 2).unwrap();
        let snapshot = store.increment_jobs(&path, -1).unwrap();
        assert_eq!(snapshot.pending_jobs, 1);
    }

    #[test]
    fn sled_job_status_tracks_pending_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let store = SledMetadataStore::open(dir.path()).unwrap();
        let path = PathBuf::from("/tmp/demo");
        let initial = store.register_repo(path.clone()).unwrap();
        assert_eq!(initial.status, RepoStatus::Idle);
        let snapshot = store.increment_jobs(&path, 1).unwrap();
        assert_eq!(snapshot.status, RepoStatus::Busy);
        let snapshot = store.increment_jobs(&path, -1).unwrap();
        assert_eq!(snapshot.status, RepoStatus::Idle);
    }

    #[test]
    fn dirty_paths_track_changes_in_memory() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        store
            .mark_dirty_path(&path, PathBuf::from("file.txt"))
            .unwrap();
        store
            .mark_dirty_path(&path, PathBuf::from("file.txt"))
            .unwrap();
        let dirty = store.drain_dirty_paths(&path).unwrap();
        assert_eq!(dirty, vec![PathBuf::from("file.txt")]);
        assert!(store.drain_dirty_paths(&path).unwrap().is_empty());
    }

    #[test]
    fn dirty_paths_persist_in_sled() {
        let dir = tempfile::tempdir().unwrap();
        let store = SledMetadataStore::open(dir.path()).unwrap();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        store
            .mark_dirty_path(&path, PathBuf::from("a.txt"))
            .unwrap();
        let drained = store.drain_dirty_paths(&path).unwrap();
        assert_eq!(drained, vec![PathBuf::from("a.txt")]);
    }

    #[test]
    fn generation_counters_increment() {
        let store = InMemoryMetadataStore::new();
        let path = PathBuf::from("/tmp/demo");
        store.register_repo(path.clone()).unwrap();
        assert_eq!(store.current_generation(&path).unwrap(), 0);
        store.bump_generation(&path).unwrap();
        assert_eq!(store.current_generation(&path).unwrap(), 1);
    }

    #[test]
    fn storage_context_prepares_directories() {
        let dir = tempfile::tempdir().unwrap();
        let context = StorageContext::new(dir.path()).unwrap();
        assert!(context.metadata_path().exists());
        context.metadata_store().unwrap();
    }
}
