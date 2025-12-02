use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, time::SystemTime};
use thiserror::Error;

/// All commands the CLI/tray can send to the daemon process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonCommand {
    RegisterRepo {
        repo_path: PathBuf,
    },
    UnregisterRepo {
        repo_path: PathBuf,
    },
    ListRepos,
    Status {
        repo_path: PathBuf,
        known_generation: Option<u64>,
    },
    QueueJob {
        repo_path: PathBuf,
        job: JobKind,
    },
    HealthCheck,
    /// Request detailed health diagnostics for a specific repository.
    RepoHealth {
        repo_path: PathBuf,
    },
    Metrics,
    FsMonitorSnapshot {
        repo_path: PathBuf,
        last_seen_generation: Option<u64>,
    },
    FetchLogs {
        repo_path: PathBuf,
        limit: usize,
    },
    /// Request graceful daemon shutdown.
    Shutdown,
}

/// Every response the daemon can emit. Real IPC will eventually serialize this
/// across async-nng sockets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DaemonResponse {
    Ack(Ack),
    RepoList(Vec<RepoSummary>),
    RepoStatus(RepoStatusDetail),
    RepoStatusUnchanged { repo_path: PathBuf, generation: u64 },
    Health(DaemonHealth),
    RepoHealth(RepoHealthDetail),
    Metrics(DaemonMetrics),
    FsMonitorSnapshot(FsMonitorSnapshot),
    Logs(Vec<LogEntry>),
    Error(String),
}

/// Lightweight acknowledgement wrapper used by multiple commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ack {
    pub message: String,
}

impl Ack {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Snapshot of daemon-level health information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonHealth {
    pub repo_count: usize,
    pub pending_jobs: usize,
    pub uptime_seconds: u64,
    pub repo_generations: Vec<RepoGeneration>,
}

/// Detailed health diagnostics for a specific repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoHealthDetail {
    pub repo_path: PathBuf,
    pub generation: u64,
    pub pending_jobs: usize,
    pub watcher_active: bool,
    pub last_event: Option<SystemTime>,
    pub dirty_path_count: usize,
    pub sled_ok: bool,
    pub needs_reconciliation: bool,
    pub throttling_active: bool,
    pub next_scheduled_job: Option<String>,
}

/// Generation token for a registered repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoGeneration {
    pub repo_path: PathBuf,
    pub generation: u64,
}

/// Snapshot of daemon metrics such as job counters and resource usage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonMetrics {
    pub jobs: HashMap<JobKind, JobMetrics>,
    pub global: GlobalMetrics,
    pub repos: Vec<RepoMetrics>,
}

impl DaemonMetrics {
    pub fn new() -> Self {
        Self {
            jobs: HashMap::new(),
            global: GlobalMetrics::default(),
            repos: Vec::new(),
        }
    }
}

/// Aggregate daemon-level statistics.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalMetrics {
    pub pending_jobs: usize,
    pub uptime_seconds: u64,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
}

/// Lightweight view into per-repository queue depth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoMetrics {
    pub repo_path: PathBuf,
    pub pending_jobs: usize,
}

/// Metadata describing a repository within the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoSummary {
    pub repo_path: PathBuf,
    pub status: RepoStatus,
    pub pending_jobs: usize,
    pub last_event: Option<SystemTime>,
    pub generation: u64,
}

impl RepoSummary {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            status: RepoStatus::Unknown,
            pending_jobs: 0,
            last_event: None,
            generation: 0,
        }
    }
}

/// Details about a repository used by `gitz status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoStatusDetail {
    pub repo_path: PathBuf,
    pub dirty_paths: Vec<PathBuf>,
    pub generation: u64,
}

/// Snapshot of paths that changed since the previous fsmonitor token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsMonitorSnapshot {
    pub repo_path: PathBuf,
    pub dirty_paths: Vec<PathBuf>,
    pub generation: u64,
}

/// A coarse view of repository health. The daemon tightens this as the
/// implementation matures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepoStatus {
    Idle,
    Busy,
    Unknown,
}

impl RepoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepoStatus::Idle => "idle",
            RepoStatus::Busy => "busy",
            RepoStatus::Unknown => "unknown",
        }
    }
}

/// Job types queued within the daemon scheduler.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobKind {
    Prefetch,
    Maintenance,
}

impl JobKind {
    pub const ALL: [JobKind; 2] = [JobKind::Prefetch, JobKind::Maintenance];

    pub fn as_str(self) -> &'static str {
        match self {
            JobKind::Prefetch => "prefetch",
            JobKind::Maintenance => "maintenance",
        }
    }
}

/// Counters describing how jobs progressed through the scheduler.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobMetrics {
    pub spawned: u64,
    pub completed: u64,
    pub failed: u64,
}

/// Streaming notifications emitted by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DaemonNotification {
    WatchEvent(WatchEventNotification),
    JobEvent(JobEventNotification),
    Log(LogEntry),
    RepoStatus(RepoStatusDetail),
}

/// A filesystem event observed for a registered repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchEventNotification {
    pub repo_path: PathBuf,
    pub path: PathBuf,
    pub kind: WatchEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WatchEventKind {
    Created,
    Modified,
    Deleted,
}

/// Lifecycle event for a background job.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobEventNotification {
    pub repo_path: PathBuf,
    pub job: JobKind,
    pub kind: JobEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobEventKind {
    Queued,
    Started,
    Completed,
    Failed,
}

/// Top-level error used when the CLI fails to talk to the daemon.
#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("daemon rejected command: {0}")]
    Rejected(String),
    #[error("transport error: {0}")]
    Transport(String),
}

/// Client-facing trait. Implementations may talk to the daemon over IPC or
/// shortcut calls in-process for tests.
#[async_trait]
pub trait DaemonService: Send + Sync {
    async fn execute(&self, command: DaemonCommand) -> Result<DaemonResponse, DaemonError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn serde_roundtrip() {
        let response = DaemonResponse::RepoStatus(RepoStatusDetail {
            repo_path: PathBuf::from("/tmp/test"),
            dirty_paths: vec![PathBuf::from("file.txt")],
            generation: 1,
        });
        let serialized = serde_json::to_string(&response).expect("serialize");
        let roundtrip: DaemonResponse =
            serde_json::from_str(&serialized).expect("deserialize from json");
        assert_eq!(response, roundtrip);
    }

    #[test]
    fn serde_roundtrip_metrics() {
        let mut jobs = HashMap::new();
        let mut counts = JobMetrics::default();
        counts.spawned = 3;
        counts.completed = 2;
        jobs.insert(JobKind::Prefetch, counts);
        let response = DaemonResponse::Metrics(DaemonMetrics {
            jobs,
            global: GlobalMetrics {
                pending_jobs: 1,
                uptime_seconds: 2,
                cpu_percent: 3.0,
                rss_bytes: 4,
            },
            repos: vec![RepoMetrics {
                repo_path: PathBuf::from("/tmp"),
                pending_jobs: 1,
            }],
        });
        let serialized = serde_json::to_string(&response).expect("serialize metrics");
        let roundtrip: DaemonResponse =
            serde_json::from_str(&serialized).expect("deserialize metrics");
        assert_eq!(response, roundtrip);
    }

    #[test]
    fn serde_roundtrip_watch_notification() {
        let notification = DaemonNotification::WatchEvent(WatchEventNotification {
            repo_path: PathBuf::from("/tmp/demo"),
            path: PathBuf::from("file.txt"),
            kind: WatchEventKind::Modified,
        });
        let serialized =
            serde_json::to_string(&notification).expect("serialize watch notification");
        let roundtrip: DaemonNotification =
            serde_json::from_str(&serialized).expect("deserialize watch notification");
        assert_eq!(notification, roundtrip);
    }

    #[test]
    fn serde_roundtrip_job_notification() {
        let notification = DaemonNotification::JobEvent(JobEventNotification {
            repo_path: PathBuf::from("/tmp/demo"),
            job: JobKind::Prefetch,
            kind: JobEventKind::Started,
        });
        let serialized = serde_json::to_string(&notification).expect("serialize job notification");
        let roundtrip: DaemonNotification =
            serde_json::from_str(&serialized).expect("deserialize job notification");
        assert_eq!(notification, roundtrip);
    }

    #[test]
    fn serde_roundtrip_status_notification() {
        let detail = RepoStatusDetail {
            repo_path: PathBuf::from("/tmp/demo"),
            dirty_paths: vec![PathBuf::from("file.txt")],
            generation: 7,
        };
        let notification = DaemonNotification::RepoStatus(detail.clone());
        let serialized =
            serde_json::to_string(&notification).expect("serialize status notification");
        let roundtrip: DaemonNotification =
            serde_json::from_str(&serialized).expect("deserialize status notification");
        assert_eq!(notification, roundtrip);
        if let DaemonNotification::RepoStatus(decoded) = roundtrip {
            assert_eq!(decoded, detail);
        } else {
            panic!("expected status notification");
        }
    }
}
/// Structured log entry emitted by the daemon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEntry {
    pub repo_path: PathBuf,
    pub message: String,
    pub timestamp: SystemTime,
}
