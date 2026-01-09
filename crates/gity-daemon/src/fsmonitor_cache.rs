//! Memory-mapped fsmonitor cache for zero-copy response delivery.
//!
//! This module provides a file-based cache that allows the fsmonitor-helper
//! to read snapshot data directly without IPC roundtrip to the daemon.
//!
//! File format:
//! ```text
//! +----------------+
//! | Header (24B)   |
//! |  - magic (4B)  |  "GITY"
//! |  - version (1B)|  1
//! |  - reserved(3B)|  0
//! |  - gen (8B)    |  generation counter (little-endian)
//! |  - count (4B)  |  number of paths (little-endian)
//! |  - total_len(4B)| total bytes of path data (little-endian)
//! +----------------+
//! | Path entries   |
//! |  - len (2B)    |  path length (little-endian)
//! |  - path (var)  |  UTF-8 path bytes
//! +----------------+
//! ```

use gity_ipc::FsMonitorSnapshot;
use sha1::{Digest, Sha1};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

const MAGIC: &[u8; 4] = b"GITY";
const VERSION: u8 = 1;
const HEADER_SIZE: usize = 24;

/// Memory-mapped fsmonitor cache for fast snapshot access.
pub struct FsMonitorCache {
    cache_dir: PathBuf,
}

impl FsMonitorCache {
    /// Create a new cache with the given directory.
    pub fn new(cache_dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&cache_dir)?;
        Ok(Self { cache_dir })
    }

    /// Generate cache file path for a repository.
    fn cache_path(&self, repo_path: &Path) -> PathBuf {
        let mut hasher = Sha1::new();
        hasher.update(repo_path.to_string_lossy().as_bytes());
        let hash = hex::encode(hasher.finalize());
        self.cache_dir.join(format!("{}.cache", &hash[..16]))
    }

    /// Write a snapshot to the cache atomically.
    pub fn write(&self, repo_path: &Path, snapshot: &FsMonitorSnapshot) -> io::Result<()> {
        let cache_path = self.cache_path(repo_path);
        let temp_path = cache_path.with_extension("tmp");

        // Calculate total size needed
        let paths_size: usize = snapshot
            .dirty_paths
            .iter()
            .map(|p| 2 + p.to_string_lossy().len())
            .sum();
        let total_size = HEADER_SIZE + paths_size;

        // Write to temp file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)?;

        let mut buffer = Vec::with_capacity(total_size);

        // Write header
        buffer.extend_from_slice(MAGIC);
        buffer.push(VERSION);
        buffer.extend_from_slice(&[0u8; 3]); // reserved
        buffer.extend_from_slice(&snapshot.generation.to_le_bytes());
        buffer.extend_from_slice(&(snapshot.dirty_paths.len() as u32).to_le_bytes());
        buffer.extend_from_slice(&(paths_size as u32).to_le_bytes());

        // Write paths
        for path in &snapshot.dirty_paths {
            let path_str = path.to_string_lossy();
            let path_bytes = path_str.as_bytes();
            buffer.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            buffer.extend_from_slice(path_bytes);
        }

        file.write_all(&buffer)?;
        file.sync_all()?;
        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &cache_path)?;

        Ok(())
    }

    /// Read a snapshot from the cache.
    pub fn read(&self, repo_path: &Path) -> io::Result<Option<FsMonitorSnapshot>> {
        let cache_path = self.cache_path(repo_path);

        let mut file = match File::open(&cache_path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        if buffer.len() < HEADER_SIZE {
            return Ok(None);
        }

        // Validate header
        if &buffer[0..4] != MAGIC {
            return Ok(None);
        }
        if buffer[4] != VERSION {
            return Ok(None);
        }

        // Parse header
        let generation = u64::from_le_bytes(buffer[8..16].try_into().unwrap());
        let count = u32::from_le_bytes(buffer[16..20].try_into().unwrap()) as usize;
        let _paths_size = u32::from_le_bytes(buffer[20..24].try_into().unwrap()) as usize;

        // Parse paths
        let mut dirty_paths = Vec::with_capacity(count);
        let mut offset = HEADER_SIZE;

        for _ in 0..count {
            if offset + 2 > buffer.len() {
                return Ok(None);
            }
            let path_len = u16::from_le_bytes(buffer[offset..offset + 2].try_into().unwrap()) as usize;
            offset += 2;

            if offset + path_len > buffer.len() {
                return Ok(None);
            }
            let path_str = String::from_utf8_lossy(&buffer[offset..offset + path_len]);
            dirty_paths.push(PathBuf::from(path_str.into_owned()));
            offset += path_len;
        }

        Ok(Some(FsMonitorSnapshot {
            repo_path: repo_path.to_path_buf(),
            dirty_paths,
            generation,
        }))
    }

    /// Read only the generation number from cache (fast path).
    pub fn read_generation(&self, repo_path: &Path) -> io::Result<Option<u64>> {
        let cache_path = self.cache_path(repo_path);

        let mut file = match File::open(&cache_path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };

        let mut header = [0u8; HEADER_SIZE];
        if file.read_exact(&mut header).is_err() {
            return Ok(None);
        }

        // Validate magic
        if &header[0..4] != MAGIC || header[4] != VERSION {
            return Ok(None);
        }

        let generation = u64::from_le_bytes(header[8..16].try_into().unwrap());
        Ok(Some(generation))
    }

    /// Remove cached snapshot for a repository.
    pub fn remove(&self, repo_path: &Path) -> io::Result<()> {
        let cache_path = self.cache_path(repo_path);
        match fs::remove_file(&cache_path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_write_and_read() {
        let temp = TempDir::new().unwrap();
        let cache = FsMonitorCache::new(temp.path().join("cache")).unwrap();

        let repo_path = PathBuf::from("/test/repo");
        let snapshot = FsMonitorSnapshot {
            repo_path: repo_path.clone(),
            dirty_paths: vec![
                PathBuf::from("src/main.rs"),
                PathBuf::from("Cargo.toml"),
            ],
            generation: 42,
        };

        cache.write(&repo_path, &snapshot).unwrap();
        let read_back = cache.read(&repo_path).unwrap().unwrap();

        assert_eq!(read_back.generation, 42);
        assert_eq!(read_back.dirty_paths.len(), 2);
        assert_eq!(read_back.dirty_paths[0], PathBuf::from("src/main.rs"));
        assert_eq!(read_back.dirty_paths[1], PathBuf::from("Cargo.toml"));
    }

    #[test]
    fn test_read_generation_only() {
        let temp = TempDir::new().unwrap();
        let cache = FsMonitorCache::new(temp.path().join("cache")).unwrap();

        let repo_path = PathBuf::from("/test/repo");
        let snapshot = FsMonitorSnapshot {
            repo_path: repo_path.clone(),
            dirty_paths: vec![PathBuf::from("file.txt")],
            generation: 123,
        };

        cache.write(&repo_path, &snapshot).unwrap();
        let gen = cache.read_generation(&repo_path).unwrap().unwrap();

        assert_eq!(gen, 123);
    }

    #[test]
    fn test_read_missing() {
        let temp = TempDir::new().unwrap();
        let cache = FsMonitorCache::new(temp.path().join("cache")).unwrap();

        let repo_path = PathBuf::from("/nonexistent/repo");
        assert!(cache.read(&repo_path).unwrap().is_none());
        assert!(cache.read_generation(&repo_path).unwrap().is_none());
    }

    #[test]
    fn test_remove() {
        let temp = TempDir::new().unwrap();
        let cache = FsMonitorCache::new(temp.path().join("cache")).unwrap();

        let repo_path = PathBuf::from("/test/repo");
        let snapshot = FsMonitorSnapshot {
            repo_path: repo_path.clone(),
            dirty_paths: vec![],
            generation: 1,
        };

        cache.write(&repo_path, &snapshot).unwrap();
        assert!(cache.read(&repo_path).unwrap().is_some());

        cache.remove(&repo_path).unwrap();
        assert!(cache.read(&repo_path).unwrap().is_none());
    }
}
