use clap::{Parser, Subcommand};
use gitz_ipc::{
    DaemonCommand, DaemonError, DaemonHealth, DaemonMetrics, DaemonResponse, DaemonService,
    FsMonitorSnapshot, JobKind, LogEntry, RepoHealthDetail, RepoStatusDetail, RepoSummary,
};
use std::{path::PathBuf, time::SystemTime};
use thiserror::Error;

/// CLI definition shared by the `gitz` binary and tests.
#[derive(Debug, Parser)]
#[command(author, version, about = "Fast Git helper daemon")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Register a repository with the daemon.
    Register { repo_path: PathBuf },
    /// Remove a repository from the daemon.
    Unregister { repo_path: PathBuf },
    /// List registered repositories.
    List {
        /// Include daemon metrics in the listing output.
        #[arg(long)]
        stats: bool,
    },
    /// Stream daemon notifications (e.g., watcher events).
    Events,
    /// Display cached status for a repository.
    Status { repo_path: PathBuf },
    /// List files changed since a generation token.
    Changed {
        repo_path: PathBuf,
        #[arg(long)]
        since: Option<u64>,
    },
    /// Print daemon logs for a repository.
    Logs {
        repo_path: PathBuf,
        #[arg(long)]
        follow: bool,
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Git fsmonitor helper entrypoint (invoked by core.fsmonitor).
    FsmonitorHelper {
        #[arg(value_parser = clap::value_parser!(u8).range(1..=2), default_value_t = 2)]
        version: u8,
        #[arg()]
        token: Option<String>,
        #[arg(long)]
        repo: Option<PathBuf>,
    },
    /// Trigger background prefetch for a repository.
    Prefetch {
        repo_path: PathBuf,
        /// Run immediately instead of queuing.
        #[arg(long)]
        now: bool,
    },
    /// Force maintenance tasks (commit-graph, GC) for a repository.
    Maintain { repo_path: PathBuf },
    /// Run health diagnostics for a repository.
    Health { repo_path: PathBuf },
    /// Launch the system tray UI.
    Tray,
    /// Talk to the daemon control plane.
    #[command(subcommand)]
    Daemon(DaemonCommands),
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommands {
    /// Start the daemon in the current process (foreground).
    Run,
    /// Start the daemon as a detached background process.
    Start,
    /// Stop a running daemon gracefully.
    Stop,
    /// Run daemon for a single repo, service queued jobs, then exit.
    Oneshot { repo_path: PathBuf },
    /// Fetch daemon health from a running instance.
    Health,
    /// Print daemon metrics and exit.
    Metrics,
    /// Request that a background job runs immediately.
    QueueJob {
        repo_path: PathBuf,
        #[arg(value_enum)]
        job: CliJobKind,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum CliJobKind {
    Prefetch,
    Maintenance,
}

impl From<CliJobKind> for JobKind {
    fn from(value: CliJobKind) -> Self {
        match value {
            CliJobKind::Prefetch => JobKind::Prefetch,
            CliJobKind::Maintenance => JobKind::Maintenance,
        }
    }
}

/// High-level action resolved from the CLI.
#[derive(Debug)]
pub enum CliAction {
    Rpc(DaemonCommand),
    List {
        stats: bool,
    },
    Logs {
        repo_path: PathBuf,
        follow: bool,
        limit: usize,
    },
    FsMonitorHelper {
        version: u8,
        token: Option<String>,
        repo: Option<PathBuf>,
    },
    StreamEvents,
    RunDaemon,
    StartDaemon,
    StopDaemon,
    OneshotDaemon {
        repo_path: PathBuf,
    },
    RunTray,
}

impl Cli {
    pub fn into_action(self) -> CliAction {
        match self.command {
            Commands::Register { repo_path } => {
                CliAction::Rpc(DaemonCommand::RegisterRepo { repo_path })
            }
            Commands::Unregister { repo_path } => {
                CliAction::Rpc(DaemonCommand::UnregisterRepo { repo_path })
            }
            Commands::List { stats } => CliAction::List { stats },
            Commands::Events => CliAction::StreamEvents,
            Commands::Changed { repo_path, since } => {
                CliAction::Rpc(DaemonCommand::FsMonitorSnapshot {
                    repo_path,
                    last_seen_generation: since,
                })
            }
            Commands::Logs {
                repo_path,
                follow,
                limit,
            } => CliAction::Logs {
                repo_path,
                follow,
                limit,
            },
            Commands::FsmonitorHelper {
                version,
                token,
                repo,
            } => CliAction::FsMonitorHelper {
                version,
                token,
                repo,
            },
            Commands::Status { repo_path } => CliAction::Rpc(DaemonCommand::Status {
                repo_path,
                known_generation: None,
            }),
            Commands::Prefetch { repo_path, now: _ } => CliAction::Rpc(DaemonCommand::QueueJob {
                repo_path,
                job: JobKind::Prefetch,
            }),
            Commands::Maintain { repo_path } => CliAction::Rpc(DaemonCommand::QueueJob {
                repo_path,
                job: JobKind::Maintenance,
            }),
            Commands::Health { repo_path } => {
                CliAction::Rpc(DaemonCommand::RepoHealth { repo_path })
            }
            Commands::Tray => CliAction::RunTray,
            Commands::Daemon(cmd) => match cmd {
                DaemonCommands::Run => CliAction::RunDaemon,
                DaemonCommands::Start => CliAction::StartDaemon,
                DaemonCommands::Stop => CliAction::StopDaemon,
                DaemonCommands::Oneshot { repo_path } => CliAction::OneshotDaemon { repo_path },
                DaemonCommands::Health => CliAction::Rpc(DaemonCommand::HealthCheck),
                DaemonCommands::Metrics => CliAction::Rpc(DaemonCommand::Metrics),
                DaemonCommands::QueueJob { repo_path, job } => {
                    CliAction::Rpc(DaemonCommand::QueueJob {
                        repo_path,
                        job: job.into(),
                    })
                }
            },
        }
    }
}

pub struct CliOutput {
    pub message: String,
}

/// Error type returned by the CLI harness.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Daemon(#[from] DaemonError),
}

pub async fn execute_rpc(
    service: &impl DaemonService,
    command: DaemonCommand,
) -> Result<CliOutput, CliError> {
    let response = service.execute(command).await?;
    Ok(CliOutput {
        message: format_response(&response),
    })
}

pub fn format_response(response: &DaemonResponse) -> String {
    match response {
        DaemonResponse::Ack(ack) => ack.message.clone(),
        DaemonResponse::RepoList(list) => format_repo_list(list),
        DaemonResponse::RepoStatus(detail) => format_repo_status(detail),
        DaemonResponse::RepoStatusUnchanged {
            repo_path,
            generation,
        } => format!(
            "{}: unchanged (generation {})",
            repo_path.display(),
            generation
        ),
        DaemonResponse::Health(health) => format_health(health),
        DaemonResponse::RepoHealth(detail) => format_repo_health(detail),
        DaemonResponse::Metrics(metrics) => format_metrics(metrics),
        DaemonResponse::FsMonitorSnapshot(snapshot) => format_fsmonitor_snapshot(snapshot),
        DaemonResponse::Logs(entries) => format_logs(entries),
        DaemonResponse::Error(msg) => msg.clone(),
    }
}

fn format_repo_list(list: &[RepoSummary]) -> String {
    if list.is_empty() {
        "no repositories registered".to_string()
    } else {
        list.iter()
            .map(format_repo_summary_line)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn format_repo_summary_line(summary: &RepoSummary) -> String {
    format!(
        "{} [{} jobs, status {}, gen {}]",
        summary.repo_path.display(),
        summary.pending_jobs,
        summary.status.as_str(),
        summary.generation
    )
}

pub fn format_repo_status(detail: &RepoStatusDetail) -> String {
    if detail.dirty_paths.is_empty() {
        format!(
            "{}: clean (generation {})",
            detail.repo_path.display(),
            detail.generation
        )
    } else {
        let mut lines = vec![format!(
            "{} (generation {}):",
            detail.repo_path.display(),
            detail.generation
        )];
        lines.extend(
            detail
                .dirty_paths
                .iter()
                .map(|path| format!("  {}", path.display())),
        );
        lines.join("\n")
    }
}

fn format_health(health: &DaemonHealth) -> String {
    let mut lines = vec![format!(
        "repos: {}, pending jobs: {}, uptime: {}s",
        health.repo_count, health.pending_jobs, health.uptime_seconds
    )];
    if !health.repo_generations.is_empty() {
        lines.push("repo generations:".into());
        for entry in &health.repo_generations {
            lines.push(format!(
                "  {} -> generation {}",
                entry.repo_path.display(),
                entry.generation
            ));
        }
    }
    lines.join("\n")
}

fn format_repo_health(detail: &RepoHealthDetail) -> String {
    let mut lines = vec![format!("Health report for {}", detail.repo_path.display())];
    lines.push(format!("  generation: {}", detail.generation));
    lines.push(format!("  pending jobs: {}", detail.pending_jobs));
    lines.push(format!(
        "  watcher: {}",
        if detail.watcher_active {
            "active"
        } else {
            "inactive"
        }
    ));
    if let Some(last_event) = detail.last_event {
        let timestamp = last_event
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|dur| dur.as_secs())
            .unwrap_or_default();
        lines.push(format!("  last event: {} (unix)", timestamp));
    } else {
        lines.push("  last event: none".into());
    }
    lines.push(format!("  dirty paths: {}", detail.dirty_path_count));
    lines.push(format!(
        "  sled integrity: {}",
        if detail.sled_ok { "ok" } else { "ERROR" }
    ));
    lines.push(format!(
        "  needs reconciliation: {}",
        if detail.needs_reconciliation {
            "yes"
        } else {
            "no"
        }
    ));
    lines.push(format!(
        "  throttling: {}",
        if detail.throttling_active {
            "active"
        } else {
            "off"
        }
    ));
    if let Some(next_job) = &detail.next_scheduled_job {
        lines.push(format!("  next scheduled job: {}", next_job));
    }
    lines.join("\n")
}

fn format_metrics(metrics: &DaemonMetrics) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "daemon: cpu={:.1}%, rss={}, uptime={}s, pending jobs={}",
        metrics.global.cpu_percent,
        format_bytes(metrics.global.rss_bytes),
        metrics.global.uptime_seconds,
        metrics.global.pending_jobs
    ));
    if !metrics.repos.is_empty() {
        lines.push("repo queue depth:".into());
        for repo in &metrics.repos {
            lines.push(format!(
                "  {} -> {} pending",
                repo.repo_path.display(),
                repo.pending_jobs
            ));
        }
    }
    lines.push("job counters:".into());
    for kind in JobKind::ALL {
        let counts = metrics.jobs.get(&kind).copied().unwrap_or_default();
        lines.push(format!("  {}", render_metric_line(kind, counts)));
    }
    let mut extras: Vec<_> = metrics
        .jobs
        .iter()
        .filter(|(kind, _)| !JobKind::ALL.contains(kind))
        .map(|(kind, counts)| format!("  {}", render_metric_line(*kind, *counts)))
        .collect();
    lines.append(&mut extras);
    lines.join("\n")
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

fn render_metric_line(kind: JobKind, counts: gitz_ipc::JobMetrics) -> String {
    format!(
        "{}: spawned={}, completed={}, failed={}",
        kind.as_str(),
        counts.spawned,
        counts.completed,
        counts.failed
    )
}

fn format_logs(entries: &[LogEntry]) -> String {
    if entries.is_empty() {
        "no log entries found".into()
    } else {
        entries
            .iter()
            .map(format_log_entry)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn format_fsmonitor_snapshot(snapshot: &FsMonitorSnapshot) -> String {
    if snapshot.dirty_paths.is_empty() {
        format!(
            "{}: no changes (generation {})",
            snapshot.repo_path.display(),
            snapshot.generation
        )
    } else {
        let mut lines = vec![format!(
            "{} (generation {}):",
            snapshot.repo_path.display(),
            snapshot.generation
        )];
        lines.extend(
            snapshot
                .dirty_paths
                .iter()
                .map(|path| format!("  {}", path.display())),
        );
        lines.join("\n")
    }
}

fn format_log_entry(entry: &LogEntry) -> String {
    let timestamp = entry
        .timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or_default();
    format!(
        "[{}] {}: {}",
        timestamp,
        entry.repo_path.display(),
        entry.message
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use gitz_ipc::{
        DaemonHealth, DaemonMetrics, DaemonResponse, FsMonitorSnapshot, JobKind, JobMetrics,
        RepoStatus, RepoStatusDetail, RepoSummary,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    #[test]
    fn cli_action_for_register() {
        let cli = Cli::parse_from(["gitz", "register", "/tmp/demo"]);
        match cli.into_action() {
            CliAction::Rpc(DaemonCommand::RegisterRepo { repo_path }) => {
                assert_eq!(repo_path, PathBuf::from("/tmp/demo"));
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_status() {
        let cli = Cli::parse_from(["gitz", "status", "/tmp/demo"]);
        match cli.into_action() {
            CliAction::Rpc(DaemonCommand::Status {
                repo_path,
                known_generation,
            }) => {
                assert_eq!(repo_path, PathBuf::from("/tmp/demo"));
                assert!(known_generation.is_none());
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_list_with_stats() {
        let cli = Cli::parse_from(["gitz", "list", "--stats"]);
        match cli.into_action() {
            CliAction::List { stats } => assert!(stats),
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_daemon_metrics() {
        let cli = Cli::parse_from(["gitz", "daemon", "metrics"]);
        match cli.into_action() {
            CliAction::Rpc(DaemonCommand::Metrics) => {}
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_events() {
        let cli = Cli::parse_from(["gitz", "events"]);
        match cli.into_action() {
            CliAction::StreamEvents => {}
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_fsmonitor_helper() {
        let cli = Cli::parse_from(["gitz", "fsmonitor-helper", "2", "123"]);
        match cli.into_action() {
            CliAction::FsMonitorHelper { version, token, .. } => {
                assert_eq!(version, 2);
                assert_eq!(token.as_deref(), Some("123"));
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn cli_action_for_logs_follow() {
        let cli = Cli::parse_from(["gitz", "logs", "/tmp/demo", "--follow", "--limit", "5"]);
        match cli.into_action() {
            CliAction::Logs {
                repo_path,
                follow,
                limit,
            } => {
                assert_eq!(repo_path, PathBuf::from("/tmp/demo"));
                assert!(follow);
                assert_eq!(limit, 5);
            }
            other => panic!("unexpected action: {other:?}"),
        }
    }

    #[test]
    fn format_fsmonitor_snapshot_lists_paths() {
        let snapshot = FsMonitorSnapshot {
            repo_path: PathBuf::from("/tmp/demo"),
            dirty_paths: vec![PathBuf::from("a.txt"), PathBuf::from("dir/b.txt")],
            generation: 3,
        };
        let text = format_fsmonitor_snapshot(&snapshot);
        assert!(text.contains("a.txt"));
        assert!(text.contains("dir/b.txt"));
        assert!(text.contains("generation 3"));
    }

    #[tokio::test]
    async fn prints_repo_list() {
        let service = TestService::new(vec![DaemonResponse::RepoList(vec![RepoSummary {
            repo_path: PathBuf::from("/tmp/demo"),
            status: RepoStatus::Idle,
            pending_jobs: 1,
            last_event: None,
            generation: 0,
        }])]);
        let output = execute_rpc(&service, DaemonCommand::ListRepos)
            .await
            .expect("list command succeeds");
        assert!(output.message.contains("/tmp/demo"));
        assert!(output.message.contains("[1 jobs, status idle, gen 0]"));
    }

    #[test]
    fn format_status_includes_dirty_paths() {
        let output = format_repo_status(&RepoStatusDetail {
            repo_path: PathBuf::from("/tmp/demo"),
            dirty_paths: vec![PathBuf::from("file.txt")],
            generation: 42,
        });
        assert!(output.contains("file.txt"));
        assert!(output.contains("generation 42"));
    }

    #[test]
    fn format_metrics_includes_counts() {
        let mut jobs = HashMap::new();
        jobs.insert(
            JobKind::Prefetch,
            JobMetrics {
                spawned: 3,
                completed: 2,
                failed: 1,
            },
        );
        let metrics = DaemonMetrics {
            jobs,
            global: gitz_ipc::GlobalMetrics {
                pending_jobs: 1,
                uptime_seconds: 2,
                cpu_percent: 12.5,
                rss_bytes: 2048,
            },
            repos: vec![gitz_ipc::RepoMetrics {
                repo_path: PathBuf::from("/tmp/demo"),
                pending_jobs: 3,
            }],
        };
        let output = format_metrics(&metrics);
        assert!(output.contains("daemon: cpu=12.5%"));
        assert!(output.contains("prefetch: spawned=3, completed=2, failed=1"));
        assert!(output.contains("maintenance: spawned=0, completed=0, failed=0"));
        assert!(output.contains("/tmp/demo -> 3 pending"));
    }

    #[test]
    fn format_health_lists_generations() {
        let health = DaemonHealth {
            repo_count: 2,
            pending_jobs: 1,
            uptime_seconds: 10,
            repo_generations: vec![
                gitz_ipc::RepoGeneration {
                    repo_path: PathBuf::from("/repo/a"),
                    generation: 5,
                },
                gitz_ipc::RepoGeneration {
                    repo_path: PathBuf::from("/repo/b"),
                    generation: 3,
                },
            ],
        };
        let output = format_health(&health);
        assert!(output.contains("repos: 2"));
        assert!(output.contains("/repo/a -> generation 5"));
        assert!(output.contains("/repo/b -> generation 3"));
    }

    struct TestService {
        responses: Arc<Mutex<Vec<DaemonResponse>>>,
    }

    impl TestService {
        fn new(responses: Vec<DaemonResponse>) -> Self {
            Self {
                responses: Arc::new(Mutex::new(responses)),
            }
        }
    }

    #[async_trait]
    impl DaemonService for TestService {
        async fn execute(&self, command: DaemonCommand) -> Result<DaemonResponse, DaemonError> {
            self.responses
                .lock()
                .map_err(|_| DaemonError::Transport("poisoned test service".into()))?
                .pop()
                .ok_or_else(|| DaemonError::Rejected(format!("no response for {command:?}")))
        }
    }
}
