mod events;
mod logs;
mod metrics;

use crate::{
    logs::LogBook,
    metrics::{MetricsRegistry, collect_resource_snapshot},
};
use async_nng::AsyncSocket;
use async_trait::async_trait;
pub use events::{NotificationServer, NotificationStream, NotificationSubscriber};
use gity_git::{RepoConfigurator, working_tree_status};
use gity_ipc::{
    Ack, DaemonCommand, DaemonError, DaemonHealth, DaemonNotification, DaemonResponse,
    DaemonService, FsMonitorSnapshot, GlobalMetrics, JobEventKind, JobEventNotification, JobKind,
    LogEntry, RepoGeneration, RepoHealthDetail, RepoMetrics, RepoStatusDetail, RepoSummary,
    WatchEventKind as IpcWatchEventKind, WatchEventNotification,
};
use gity_storage::{MetadataStore, RepoMetadata};
use gity_watch::{
    NotifyWatcher, WatchError, WatchEvent, WatchEventKind, WatchHandleRef, WatcherRef,
};
use nng::Protocol;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime},
};
use thiserror::Error;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::{
    process::Command,
    select,
    sync::{Notify, mpsc},
    task::JoinHandle,
    time::sleep,
};
use tracing::{error, info, warn};

type SharedDaemon<S> = Arc<Mutex<Daemon<S>>>;
type NotificationSender = mpsc::UnboundedSender<DaemonNotification>;

/// Scheduler holding a FIFO queue of work. Priority/backoff can layer on later.
#[derive(Default)]
struct Scheduler {
    queue: VecDeque<QueuedJob>,
}

#[derive(Clone)]
struct QueuedJob {
    repo_path: PathBuf,
    kind: JobKind,
}

impl Scheduler {
    fn enqueue(&mut self, repo_path: PathBuf, job: JobKind) {
        self.queue.push_back(QueuedJob {
            repo_path,
            kind: job,
        });
    }

    fn len(&self) -> usize {
        self.queue.len()
    }

    fn next_job(&mut self) -> Option<QueuedJob> {
        self.queue.pop_front()
    }
}

/// Core daemon logic. IPC layers wrap this struct.
pub struct Daemon<S: MetadataStore> {
    store: S,
    scheduler: Scheduler,
    started_at: Instant,
    metrics: MetricsRegistry,
    notifications: Option<NotificationSender>,
    fsmonitor_helper: Option<String>,
    logs: LogBook,
}

impl<S: MetadataStore> Daemon<S> {
    pub fn new(store: S) -> Self {
        Self::with_components(
            store,
            MetricsRegistry::default(),
            None,
            None,
            LogBook::new(200),
        )
    }

    pub fn with_metrics(store: S, metrics: MetricsRegistry) -> Self {
        Self::with_components(store, metrics, None, None, LogBook::new(200))
    }

    pub fn with_components(
        store: S,
        metrics: MetricsRegistry,
        notifications: Option<NotificationSender>,
        fsmonitor_helper: Option<String>,
        logs: LogBook,
    ) -> Self {
        Self {
            store,
            scheduler: Scheduler::default(),
            started_at: Instant::now(),
            metrics,
            notifications,
            fsmonitor_helper,
            logs,
        }
    }

    fn handle(&mut self, command: DaemonCommand) -> DaemonResponse {
        match command {
            DaemonCommand::RegisterRepo { repo_path } => {
                match self.store.register_repo(repo_path.clone()) {
                    Ok(_) => match configure_repo(&repo_path, self.fsmonitor_helper.as_deref()) {
                        Ok(_) => DaemonResponse::Ack(Ack::new("registered")),
                        Err(err) => err,
                    },
                    Err(err) => err_response(err),
                }
            }
            DaemonCommand::UnregisterRepo { repo_path } => {
                match self.store.unregister_repo(&repo_path) {
                    Ok(Some(_)) => match clear_repo_config(&repo_path) {
                        Ok(_) => DaemonResponse::Ack(Ack::new("unregistered")),
                        Err(err) => err,
                    },
                    Ok(None) => DaemonResponse::Error(format!(
                        "repository not registered: {}",
                        repo_path.display()
                    )),
                    Err(err) => err_response(err),
                }
            }
            DaemonCommand::ListRepos => match self.store.list_repos() {
                Ok(repos) => {
                    DaemonResponse::RepoList(repos.into_iter().map(repo_summary).collect())
                }
                Err(err) => err_response(err),
            },
            DaemonCommand::Status {
                repo_path,
                known_generation,
            } => {
                let current_generation = match self.store.current_generation(&repo_path) {
                    Ok(current) => current,
                    Err(err) => return err_response(err),
                };
                let dirty_paths = match self.store.drain_dirty_paths(&repo_path) {
                    Ok(paths) => paths,
                    Err(err) => return err_response(err),
                };
                if dirty_paths.is_empty()
                    && known_generation
                        .map(|generation| generation == current_generation)
                        .unwrap_or(false)
                {
                    return DaemonResponse::RepoStatusUnchanged {
                        repo_path,
                        generation: current_generation,
                    };
                }
                let generation = match self.store.bump_generation(&repo_path) {
                    Ok(new_generation) => new_generation,
                    Err(err) => return err_response(err),
                };
                match self.compute_status(&repo_path, dirty_paths, generation) {
                    Ok(detail) => {
                        self.emit_status_notification(&detail);
                        DaemonResponse::RepoStatus(detail)
                    }
                    Err(err) => err,
                }
            }
            DaemonCommand::QueueJob { repo_path, job } => {
                self.scheduler.enqueue(repo_path.clone(), job.clone());
                send_job_notification(
                    &self.notifications,
                    &repo_path,
                    job.clone(),
                    JobEventKind::Queued,
                );
                match self.store.increment_jobs(&repo_path, 1) {
                    Ok(_) => DaemonResponse::Ack(Ack::new(format!("queued {:?}", job))),
                    Err(err) => err_response(err),
                }
            }
            DaemonCommand::HealthCheck => match self.store.list_repos() {
                Ok(repos) => {
                    let generations = repos
                        .iter()
                        .map(|meta| RepoGeneration {
                            repo_path: meta.repo_path.clone(),
                            generation: meta.generation,
                        })
                        .collect();
                    DaemonResponse::Health(DaemonHealth {
                        repo_count: repos.len(),
                        pending_jobs: self.scheduler.len(),
                        uptime_seconds: self.started_at.elapsed().as_secs(),
                        repo_generations: generations,
                    })
                }
                Err(err) => err_response(err),
            },
            DaemonCommand::Metrics => match self.store.list_repos() {
                Ok(repos) => {
                    let resource = collect_resource_snapshot();
                    let mut snapshot = self.metrics.snapshot();
                    snapshot.global = GlobalMetrics {
                        pending_jobs: self.scheduler.len(),
                        uptime_seconds: self.started_at.elapsed().as_secs(),
                        cpu_percent: resource.cpu_percent,
                        rss_bytes: resource.rss_bytes,
                    };
                    snapshot.repos = repos
                        .into_iter()
                        .map(|meta| RepoMetrics {
                            repo_path: meta.repo_path,
                            pending_jobs: meta.pending_jobs,
                        })
                        .collect();
                    DaemonResponse::Metrics(snapshot)
                }
                Err(err) => err_response(err),
            },
            DaemonCommand::FsMonitorSnapshot {
                repo_path,
                last_seen_generation,
            } => {
                let current_generation = match self.store.current_generation(&repo_path) {
                    Ok(current) => current,
                    Err(err) => return err_response(err),
                };
                let dirty_paths = match self.store.drain_dirty_paths(&repo_path) {
                    Ok(paths) => paths,
                    Err(err) => return err_response(err),
                };
                if dirty_paths.is_empty()
                    && last_seen_generation
                        .map(|known| known == current_generation)
                        .unwrap_or(false)
                {
                    return DaemonResponse::FsMonitorSnapshot(FsMonitorSnapshot {
                        repo_path,
                        dirty_paths: Vec::new(),
                        generation: current_generation,
                    });
                }
                let generation = match self.store.bump_generation(&repo_path) {
                    Ok(new_generation) => new_generation,
                    Err(err) => return err_response(err),
                };
                // Filter out .git internal paths - fsmonitor only wants working tree files
                let working_tree_paths = filter_working_tree_paths(dirty_paths);
                DaemonResponse::FsMonitorSnapshot(FsMonitorSnapshot {
                    repo_path,
                    dirty_paths: working_tree_paths,
                    generation,
                })
            }
            DaemonCommand::FetchLogs { repo_path, limit } => {
                let entries = self.logs.recent(&repo_path, limit);
                DaemonResponse::Logs(entries)
            }
            DaemonCommand::RepoHealth { repo_path } => self.compute_repo_health(&repo_path),
            DaemonCommand::Shutdown => DaemonResponse::Ack(Ack::new("shutdown requested")),
        }
    }

    fn compute_repo_health(&self, repo_path: &Path) -> DaemonResponse {
        let meta = match self.store.get_repo(repo_path) {
            Ok(Some(meta)) => meta,
            Ok(None) => {
                return DaemonResponse::Error(format!(
                    "repository not registered: {}",
                    repo_path.display()
                ));
            }
            Err(err) => return err_response(err),
        };
        let dirty_path_count = self.store.dirty_path_count(repo_path).unwrap_or(0);
        DaemonResponse::RepoHealth(RepoHealthDetail {
            repo_path: repo_path.to_path_buf(),
            generation: meta.generation,
            pending_jobs: meta.pending_jobs,
            watcher_active: true, // Will be updated by runtime
            last_event: meta.last_event,
            dirty_path_count,
            sled_ok: true, // Sled integrity check placeholder
            needs_reconciliation: meta.needs_reconciliation.unwrap_or(false),
            throttling_active: false, // Resource monitor integration
            next_scheduled_job: None,
        })
    }

    fn next_job(&mut self) -> Option<QueuedJob> {
        self.scheduler.next_job()
    }

    fn mark_job_completed(&mut self, job: &QueuedJob) {
        let now = SystemTime::now();
        let _ = self.store.increment_jobs(&job.repo_path, -1);
        let _ = self.store.record_event(&job.repo_path, now);
    }

    fn emit_status_notification(&self, detail: &RepoStatusDetail) {
        if let Some(tx) = &self.notifications {
            let _ = tx.send(DaemonNotification::RepoStatus(detail.clone()));
        }
    }
}

fn repo_summary(meta: RepoMetadata) -> RepoSummary {
    RepoSummary {
        repo_path: meta.repo_path,
        status: meta.status,
        pending_jobs: meta.pending_jobs,
        last_event: meta.last_event,
        generation: meta.generation,
    }
}

fn err_response(err: impl ToString) -> DaemonResponse {
    DaemonResponse::Error(err.to_string())
}

fn configure_repo(path: &Path, helper: Option<&str>) -> Result<(), DaemonResponse> {
    RepoConfigurator::open(path)
        .and_then(|repo| repo.apply_performance_settings(helper))
        .map_err(|err| DaemonResponse::Error(format!("failed to configure repo: {err}")))
}

fn clear_repo_config(path: &Path) -> Result<(), DaemonResponse> {
    RepoConfigurator::open(path)
        .and_then(|repo| repo.clear_performance_settings())
        .map_err(|err| DaemonResponse::Error(format!("failed to clear repo config: {err}")))
}

impl<S: MetadataStore> Daemon<S> {
    fn handle_watch_event(&mut self, repo_path: &Path, event: &WatchEvent) {
        let now = SystemTime::now();
        let _ = self.store.record_event(repo_path, now);
        if should_track(&event.kind) {
            if let Some(relative) = relative_path(repo_path, &event.path) {
                let _ = self.store.mark_dirty_path(repo_path, relative);
            }
        } else if let WatchEventKind::Deleted = event.kind {
            let _ = self.store.mark_dirty_path(repo_path, PathBuf::from("."));
        }
    }

    fn repo_paths(&self) -> Vec<PathBuf> {
        self.store
            .list_repos()
            .map(|repos| repos.into_iter().map(|meta| meta.repo_path).collect())
            .unwrap_or_default()
    }

    fn compute_status(
        &self,
        repo_path: &Path,
        paths: Vec<PathBuf>,
        generation: u64,
    ) -> Result<RepoStatusDetail, DaemonResponse> {
        let dirty_paths = working_tree_status(repo_path, &paths)
            .map_err(|err| DaemonResponse::Error(format!("git status failed: {err}")))?;
        Ok(RepoStatusDetail {
            repo_path: repo_path.to_path_buf(),
            dirty_paths,
            generation,
        })
    }
}

fn relative_path(repo_path: &Path, path: &Path) -> Option<PathBuf> {
    if let Ok(relative) = path.strip_prefix(repo_path) {
        if relative.as_os_str().is_empty() {
            None
        } else {
            Some(relative.to_path_buf())
        }
    } else {
        Some(path.to_path_buf())
    }
}

/// Returns true if the path is inside the `.git` directory.
/// Git's fsmonitor only wants working tree paths, not internal Git files.
fn is_git_internal_path(path: &Path) -> bool {
    path.components().any(|c| c.as_os_str() == ".git")
}

/// Filter out `.git` internal paths for fsmonitor output.
fn filter_working_tree_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths
        .into_iter()
        .filter(|p| !is_git_internal_path(p))
        .collect()
}

fn should_track(kind: &WatchEventKind) -> bool {
    matches!(
        kind,
        WatchEventKind::Created | WatchEventKind::Modified | WatchEventKind::Deleted
    )
}

fn map_watch_kind(kind: &WatchEventKind) -> IpcWatchEventKind {
    match kind {
        WatchEventKind::Created => IpcWatchEventKind::Created,
        WatchEventKind::Modified => IpcWatchEventKind::Modified,
        WatchEventKind::Deleted => IpcWatchEventKind::Deleted,
    }
}

fn send_job_notification(
    notifications: &Option<NotificationSender>,
    repo_path: &Path,
    job: JobKind,
    kind: JobEventKind,
) {
    if let Some(tx) = notifications {
        let _ = tx.send(DaemonNotification::JobEvent(JobEventNotification {
            repo_path: repo_path.to_path_buf(),
            job,
            kind,
        }));
    }
}

fn send_log_notification(notifications: &Option<NotificationSender>, entry: &LogEntry) {
    if let Some(tx) = notifications {
        let _ = tx.send(DaemonNotification::Log(entry.clone()));
    }
}

/// Signal shared by runtime/server tasks to coordinate shutdown.
#[derive(Clone)]
pub struct Shutdown {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Shutdown {
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    pub fn shutdown(&self) {
        if !self.flag.swap(true, Ordering::SeqCst) {
            self.notify.notify_waiters();
        }
    }

    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    pub async fn wait(&self) {
        if self.is_shutdown() {
            return;
        }
        self.notify.notified().await;
    }
}

/// Configuration for idle-time scheduling.
#[derive(Clone)]
pub struct IdleScheduleConfig {
    /// Duration of inactivity before triggering prefetch.
    pub prefetch_idle_secs: u64,
    /// Duration of inactivity before triggering maintenance.
    pub maintenance_idle_secs: u64,
    /// Whether idle scheduling is enabled.
    pub enabled: bool,
}

impl Default for IdleScheduleConfig {
    fn default() -> Self {
        Self {
            prefetch_idle_secs: 300,    // 5 minutes
            maintenance_idle_secs: 600, // 10 minutes
            enabled: true,
        }
    }
}

/// Tracks per-repo idle state for scheduling.
struct RepoIdleState {
    last_activity: Instant,
    prefetch_scheduled: bool,
    maintenance_scheduled: bool,
}

impl RepoIdleState {
    fn new() -> Self {
        Self {
            last_activity: Instant::now(),
            prefetch_scheduled: false,
            maintenance_scheduled: false,
        }
    }

    fn touch(&mut self) {
        self.last_activity = Instant::now();
        self.prefetch_scheduled = false;
        self.maintenance_scheduled = false;
    }

    fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Background scheduler loop.
pub struct Runtime<S: MetadataStore> {
    daemon: SharedDaemon<S>,
    watcher: WatcherRef,
    active_watchers: Arc<Mutex<HashMap<PathBuf, WatchRegistration>>>,
    event_tx: mpsc::UnboundedSender<WatchEventEnvelope>,
    event_rx: Mutex<mpsc::UnboundedReceiver<WatchEventEnvelope>>,
    shutdown: Shutdown,
    poll_interval: Duration,
    metrics: MetricsRegistry,
    notifications: Option<NotificationSender>,
    logs: LogBook,
    idle_config: IdleScheduleConfig,
    idle_states: Arc<Mutex<HashMap<PathBuf, RepoIdleState>>>,
}

impl<S: MetadataStore> Runtime<S> {
    pub fn new(store: S, log_tree: Option<sled::Tree>) -> Self {
        Self::with_watcher_and_notifications(
            store,
            Arc::new(NotifyWatcher::new()),
            None,
            None,
            log_tree,
        )
    }

    pub fn with_watcher(store: S, watcher: WatcherRef, log_tree: Option<sled::Tree>) -> Self {
        Self::with_watcher_and_notifications(store, watcher, None, None, log_tree)
    }

    pub fn with_notifications(
        store: S,
        notifications: Option<NotificationSender>,
        fsmonitor_helper: Option<String>,
        log_tree: Option<sled::Tree>,
    ) -> Self {
        Self::with_watcher_and_notifications(
            store,
            Arc::new(NotifyWatcher::new()),
            notifications,
            fsmonitor_helper,
            log_tree,
        )
    }

    pub fn with_watcher_and_notifications(
        store: S,
        watcher: WatcherRef,
        notifications: Option<NotificationSender>,
        fsmonitor_helper: Option<String>,
        log_tree: Option<sled::Tree>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let metrics = MetricsRegistry::default();
        let logs = log_tree
            .map(|tree| LogBook::with_persistence(200, tree))
            .unwrap_or_else(|| LogBook::new(200));
        let daemon_notifications = notifications.clone();
        Self {
            daemon: Arc::new(Mutex::new(Daemon::with_components(
                store,
                metrics.clone(),
                daemon_notifications,
                fsmonitor_helper,
                logs.clone(),
            ))),
            watcher,
            active_watchers: Arc::new(Mutex::new(HashMap::new())),
            event_tx,
            event_rx: Mutex::new(event_rx),
            shutdown: Shutdown::new(),
            poll_interval: Duration::from_millis(250),
            metrics,
            notifications,
            logs,
            idle_config: IdleScheduleConfig::default(),
            idle_states: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn shared(&self) -> SharedDaemon<S> {
        Arc::clone(&self.daemon)
    }

    pub fn shutdown_signal(&self) -> Shutdown {
        self.shutdown.clone()
    }

    pub fn service_handle(&self) -> InProcessService<S> {
        InProcessService::from_shared(self.shared())
    }

    pub async fn run(self) {
        // Check for repos that need reconciliation on startup
        self.check_reconciliation_needed();

        while !self.shutdown.is_shutdown() {
            self.drain_watch_events();
            if let Some(job) = self.next_job_to_run() {
                self.spawn_job(job);
            }
            self.check_idle_schedules();
            if let Err(err) = self.sync_watchers().await {
                eprintln!("failed to synchronize watchers: {err}");
            }
            sleep(self.poll_interval).await;
        }
        self.stop_all_watchers();
    }

    /// Check for repos that need reconciliation after daemon restart/downtime.
    fn check_reconciliation_needed(&self) {
        let repos = self.current_repo_paths();
        let now = SystemTime::now();
        let max_gap = Duration::from_secs(300); // 5 minute gap triggers reconciliation

        for repo_path in repos {
            let needs_reconciliation = if let Ok(guard) = self.daemon.lock() {
                match guard.store.get_repo(&repo_path) {
                    Ok(Some(meta)) => {
                        if let Some(last_event) = meta.last_event {
                            match now.duration_since(last_event) {
                                Ok(gap) => gap > max_gap,
                                Err(_) => false,
                            }
                        } else {
                            // No last event recorded, assume fresh registration
                            false
                        }
                    }
                    _ => false,
                }
            } else {
                continue;
            };

            if needs_reconciliation {
                self.schedule_reconciliation(&repo_path);
            }
        }
    }

    /// Schedule a reconciliation scan for a repository.
    fn schedule_reconciliation(&self, repo_path: &Path) {
        let entry = self.logs.record(
            repo_path,
            "reconciliation needed after daemon downtime - scheduling status refresh",
        );
        send_log_notification(&self.notifications, &entry);

        // Mark the repo as needing reconciliation
        if let Ok(guard) = self.daemon.lock() {
            let _ = guard.store.set_needs_reconciliation(repo_path, true);
        }

        // Mark the entire repo as dirty to force a full status check
        if let Ok(guard) = self.daemon.lock() {
            let _ = guard.store.mark_dirty_path(repo_path, PathBuf::from("."));
        }

        info!("scheduled reconciliation scan for {}", repo_path.display());
    }

    fn check_idle_schedules(&self) {
        if !self.idle_config.enabled {
            return;
        }

        let repos = self.current_repo_paths();
        let prefetch_threshold = Duration::from_secs(self.idle_config.prefetch_idle_secs);
        let maintenance_threshold = Duration::from_secs(self.idle_config.maintenance_idle_secs);

        let mut states = match self.idle_states.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        for repo in repos {
            let state = states
                .entry(repo.clone())
                .or_insert_with(RepoIdleState::new);
            let idle = state.idle_duration();

            // Schedule prefetch if idle long enough
            if idle >= prefetch_threshold && !state.prefetch_scheduled {
                if let Ok(mut guard) = self.daemon.lock() {
                    guard.scheduler.enqueue(repo.clone(), JobKind::Prefetch);
                    send_job_notification(
                        &self.notifications,
                        &repo,
                        JobKind::Prefetch,
                        JobEventKind::Queued,
                    );
                    let entry = self.logs.record(&repo, "idle prefetch scheduled");
                    send_log_notification(&self.notifications, &entry);
                }
                state.prefetch_scheduled = true;
            }

            // Schedule maintenance if idle even longer
            if idle >= maintenance_threshold && !state.maintenance_scheduled {
                if let Ok(mut guard) = self.daemon.lock() {
                    guard.scheduler.enqueue(repo.clone(), JobKind::Maintenance);
                    send_job_notification(
                        &self.notifications,
                        &repo,
                        JobKind::Maintenance,
                        JobEventKind::Queued,
                    );
                    let entry = self.logs.record(&repo, "idle maintenance scheduled");
                    send_log_notification(&self.notifications, &entry);
                }
                state.maintenance_scheduled = true;
            }
        }

        // Clean up states for repos that are no longer registered
        let repo_set: HashSet<_> = self.current_repo_paths().into_iter().collect();
        states.retain(|path, _| repo_set.contains(path));
    }

    fn touch_repo_activity(&self, repo_path: &Path) {
        if let Ok(mut states) = self.idle_states.lock() {
            if let Some(state) = states.get_mut(repo_path) {
                state.touch();
            }
        }
    }

    fn next_job_to_run(&self) -> Option<QueuedJob> {
        if let Ok(mut guard) = self.daemon.lock() {
            guard.next_job()
        } else {
            None
        }
    }

    async fn sync_watchers(&self) -> Result<(), WatchError> {
        let desired = self.current_repo_paths();
        let existing: HashSet<PathBuf> = self
            .active_watchers
            .lock()
            .map_err(|_| WatchError::Backend("watcher map poisoned".into()))?
            .keys()
            .cloned()
            .collect();

        for repo in desired.iter() {
            if !existing.contains(repo) {
                if let Err(err) = self.start_watcher(repo.clone()).await {
                    eprintln!("failed to start watcher for {}: {err}", repo.display());
                }
            }
        }

        let desired_set: HashSet<PathBuf> = desired.into_iter().collect();
        let to_remove: Vec<PathBuf> = self
            .active_watchers
            .lock()
            .map_err(|_| WatchError::Backend("watcher map poisoned".into()))?
            .keys()
            .filter(|path| !desired_set.contains(*path))
            .cloned()
            .collect();

        for repo in to_remove {
            self.stop_watcher(&repo);
        }

        Ok(())
    }

    async fn start_watcher(&self, repo_path: PathBuf) -> Result<(), WatchError> {
        let subscription = self.watcher.watch(repo_path.clone()).await?;
        let (handle, mut receiver) = subscription.into_parts();
        let event_tx = self.event_tx.clone();
        let repo_for_task = repo_path.clone();
        let task = tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                if event_tx
                    .send(WatchEventEnvelope {
                        repo_path: repo_for_task.clone(),
                        event,
                    })
                    .is_err()
                {
                    warn!("watch event channel closed");
                    break;
                }
            }
        });
        self.active_watchers
            .lock()
            .map_err(|_| WatchError::Backend("watcher map poisoned".into()))?
            .insert(repo_path, WatchRegistration { handle, task });
        Ok(())
    }

    fn stop_watcher(&self, repo_path: &Path) {
        if let Ok(mut watchers) = self.active_watchers.lock() {
            if let Some(registration) = watchers.remove(repo_path) {
                registration.handle.stop();
                registration.task.abort();
            }
        }
    }

    fn stop_all_watchers(&self) {
        if let Ok(mut watchers) = self.active_watchers.lock() {
            for (_path, registration) in watchers.drain() {
                registration.handle.stop();
                registration.task.abort();
            }
        }
    }

    fn current_repo_paths(&self) -> Vec<PathBuf> {
        if let Ok(guard) = self.daemon.lock() {
            guard.repo_paths()
        } else {
            Vec::new()
        }
    }

    fn spawn_job(&self, job: QueuedJob) {
        let daemon = Arc::clone(&self.daemon);
        let metrics = self.metrics.clone();
        metrics.record_job_spawned(job.kind);
        let notifications = self.notifications.clone();
        let logs = self.logs.clone();
        let entry = logs.record(&job.repo_path, format!("job {:?} started", job.kind));
        send_log_notification(&notifications, &entry);
        send_job_notification(
            &notifications,
            &job.repo_path,
            job.kind,
            JobEventKind::Started,
        );
        tokio::spawn(async move {
            let start = Instant::now();
            let result = JobExecutor::run(&job).await;
            let duration = start.elapsed();
            match result {
                Ok(_) => {
                    metrics.record_job_completed(job.kind);
                    send_job_notification(
                        &notifications,
                        &job.repo_path,
                        job.kind,
                        JobEventKind::Completed,
                    );
                    let entry = logs.record(
                        &job.repo_path,
                        format!("job {:?} completed in {:?}", job.kind, duration),
                    );
                    send_log_notification(&notifications, &entry);
                    info!(
                        "job {:?} for {} finished in {:?}",
                        job.kind,
                        job.repo_path.display(),
                        duration
                    );
                }
                Err(err) => {
                    metrics.record_job_failed(job.kind);
                    send_job_notification(
                        &notifications,
                        &job.repo_path,
                        job.kind,
                        JobEventKind::Failed,
                    );
                    let entry = logs.record(
                        &job.repo_path,
                        format!("job {:?} failed after {:?}: {err}", job.kind, duration),
                    );
                    send_log_notification(&notifications, &entry);
                    error!(
                        "job {:?} for {} failed after {:?}: {err}",
                        job.kind,
                        job.repo_path.display(),
                        duration
                    );
                    send_log_notification(
                        &notifications,
                        &LogEntry {
                            repo_path: job.repo_path.clone(),
                            message: format!(
                                "job {:?} failed after {:?}: {err}",
                                job.kind, duration
                            ),
                            timestamp: SystemTime::now(),
                        },
                    );
                }
            }
            if let Ok(mut guard) = daemon.lock() {
                guard.mark_job_completed(&job);
            }
        });
    }

    #[cfg(test)]
    pub(crate) fn watcher_state(&self) -> Arc<Mutex<HashMap<PathBuf, WatchRegistration>>> {
        Arc::clone(&self.active_watchers)
    }

    fn drain_watch_events(&self) {
        let mut receiver = match self.event_rx.lock() {
            Ok(rx) => rx,
            Err(_) => {
                warn!("watch event receiver poisoned");
                return;
            }
        };

        loop {
            match receiver.try_recv() {
                Ok(envelope) => {
                    // Reset idle timer for this repo
                    self.touch_repo_activity(&envelope.repo_path);
                    if let Ok(mut guard) = self.daemon.lock() {
                        guard.handle_watch_event(&envelope.repo_path, &envelope.event);
                        self.emit_watch_notification(&envelope.repo_path, &envelope.event);
                        self.record_log(
                            &envelope.repo_path,
                            format!(
                                "watch {:?} {}",
                                envelope.event.kind,
                                envelope.event.path.display()
                            ),
                        );
                    } else {
                        warn!("failed to lock daemon for watch event");
                        break;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    warn!("watch event channel disconnected");
                    break;
                }
            }
        }
    }

    fn emit_watch_notification(&self, repo_path: &Path, event: &WatchEvent) {
        if let Some(tx) = &self.notifications {
            let path = relative_path(repo_path, &event.path).unwrap_or_else(|| event.path.clone());
            let notification = DaemonNotification::WatchEvent(WatchEventNotification {
                repo_path: repo_path.to_path_buf(),
                path,
                kind: map_watch_kind(&event.kind),
            });
            let _ = tx.send(notification);
        }
    }

    fn record_log(&self, repo_path: &Path, message: impl Into<String>) {
        let entry = self.logs.record(repo_path, message);
        send_log_notification(&self.notifications, &entry);
    }
}

struct WatchRegistration {
    handle: WatchHandleRef,
    task: JoinHandle<()>,
}

struct JobExecutor;

impl JobExecutor {
    async fn run(job: &QueuedJob) -> Result<(), JobExecutionError> {
        match job.kind {
            JobKind::Prefetch => {
                // Use git maintenance's prefetch task which:
                // - Fetches into refs/prefetch/ namespace (doesn't update local refs)
                // - Is safe to run in background without disrupting user
                // - Handles multiple remotes correctly
                Self::run_git(
                    &job.repo_path,
                    &["maintenance", "run", "--task=prefetch", "--quiet"],
                )
                .await
            }
            JobKind::Maintenance => {
                // Run maintenance with --auto to let Git decide what needs running
                // based on repository state (loose object count, commit-graph age, etc.)
                // This runs: commit-graph, loose-objects, incremental-repack as needed
                Self::run_git(&job.repo_path, &["maintenance", "run", "--auto", "--quiet"]).await
            }
        }
    }

    async fn run_git(repo_path: &Path, args: &[&str]) -> Result<(), JobExecutionError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(repo_path)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(JobExecutionError::Io)?;

        if output.status.success() {
            Ok(())
        } else {
            // Log stderr for debugging but don't fail on expected errors
            // (e.g., no remote configured, already up-to-date)
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No remote") || stderr.contains("not a git repository") {
                // Expected for local-only repos
                Ok(())
            } else {
                Err(JobExecutionError::Exit(output.status.code().unwrap_or(-1)))
            }
        }
    }
}

#[derive(Debug, Error)]
enum JobExecutionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git command exited with status {0}")]
    Exit(i32),
}

#[derive(Clone)]
struct WatchEventEnvelope {
    repo_path: PathBuf,
    event: WatchEvent,
}

/// Shared in-process service used by tests.
pub struct InProcessService<S: MetadataStore> {
    inner: SharedDaemon<S>,
}

impl<S: MetadataStore> InProcessService<S> {
    pub fn new(store: S) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Daemon::new(store))),
        }
    }

    pub fn from_shared(inner: SharedDaemon<S>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<S: MetadataStore> DaemonService for InProcessService<S> {
    async fn execute(&self, command: DaemonCommand) -> Result<DaemonResponse, DaemonError> {
        let mut guard = self
            .inner
            .lock()
            .map_err(|_| DaemonError::Transport("failed to acquire daemon lock".to_string()))?;
        Ok(guard.handle(command))
    }
}

/// IPC server using async-nng REP sockets.
pub struct NngServer<S: MetadataStore> {
    address: String,
    daemon: SharedDaemon<S>,
    shutdown: Shutdown,
}

impl<S: MetadataStore> NngServer<S> {
    pub fn new(address: impl Into<String>, daemon: SharedDaemon<S>, shutdown: Shutdown) -> Self {
        Self {
            address: address.into(),
            daemon,
            shutdown,
        }
    }

    pub async fn run(&self) -> Result<(), ServerError> {
        let socket = nng::Socket::new(Protocol::Rep0)?;
        socket.listen(&self.address)?;
        let mut async_socket = AsyncSocket::try_from(socket)?;

        loop {
            select! {
                _ = self.shutdown.wait() => break,
                recv = async_socket.receive(None) => {
                    let message = match recv {
                        Ok(msg) => msg,
                        Err(nng::Error::Canceled) => continue,
                        Err(err) => return Err(ServerError::Socket(err)),
                    };
                    let reply = self.process_message(message)?;
                    async_socket
                        .send(reply, None)
                        .await
                        .map_err(|(_, err)| ServerError::Socket(err))?;
                }
            }
        }

        Ok(())
    }

    fn process_message(&self, message: nng::Message) -> Result<nng::Message, ServerError> {
        let command: DaemonCommand = bincode::deserialize(message.as_slice())
            .map_err(|err| ServerError::Codec(err.to_string()))?;
        let response = {
            let mut guard = self.daemon.lock().map_err(|_| ServerError::Poisoned)?;
            guard.handle(command)
        };
        let payload =
            bincode::serialize(&response).map_err(|err| ServerError::Codec(err.to_string()))?;
        let mut reply = nng::Message::new();
        reply.push_back(&payload);
        Ok(reply)
    }
}

/// IPC client implementation using async-nng REQ sockets.
pub struct NngClient {
    address: String,
}

impl NngClient {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    async fn request(&self, command: DaemonCommand) -> Result<DaemonResponse, DaemonError> {
        let socket = nng::Socket::new(Protocol::Req0).map_err(map_client_error)?;
        socket.dial(&self.address).map_err(map_client_error)?;
        let mut async_socket = AsyncSocket::try_from(socket).map_err(map_client_error)?;

        let payload =
            bincode::serialize(&command).map_err(|err| DaemonError::Transport(err.to_string()))?;
        let mut message = nng::Message::new();
        message.push_back(&payload);
        async_socket
            .send(message, None)
            .await
            .map_err(|(_, err)| map_client_error(err))?;

        let reply = async_socket.receive(None).await.map_err(map_client_error)?;
        let response: DaemonResponse = bincode::deserialize(reply.as_slice())
            .map_err(|err| DaemonError::Transport(err.to_string()))?;
        Ok(response)
    }
}

#[async_trait]
impl DaemonService for NngClient {
    async fn execute(&self, command: DaemonCommand) -> Result<DaemonResponse, DaemonError> {
        self.request(command).await
    }
}

fn map_client_error(err: nng::Error) -> DaemonError {
    DaemonError::Transport(err.to_string())
}

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("nng error: {0}")]
    Socket(#[from] nng::Error),
    #[error("codec error: {0}")]
    Codec(String),
    #[error("daemon lock poisoned")]
    Poisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use gity_storage::InMemoryMetadataStore;
    use gity_watch::{ManualWatchHandle, ManualWatcher, WatchEvent, WatchEventKind};
    use std::time::Duration;
    use tempfile::tempdir;

    #[test]
    fn scheduler_drops_completed_jobs() {
        let store = InMemoryMetadataStore::new();
        let mut daemon = Daemon::new(store);
        let (_repo_dir, repo) = init_temp_repo();
        assert!(matches!(
            daemon.handle(DaemonCommand::RegisterRepo {
                repo_path: repo.clone()
            }),
            DaemonResponse::Ack(_)
        ));
        daemon.handle(DaemonCommand::QueueJob {
            repo_path: repo.clone(),
            job: JobKind::Prefetch,
        });
        if let Some(job) = daemon.next_job() {
            daemon.mark_job_completed(&job);
        }
        match daemon.handle(DaemonCommand::ListRepos) {
            DaemonResponse::RepoList(entries) => assert_eq!(entries[0].pending_jobs, 0),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn metrics_snapshot_reflects_registry() {
        let store = InMemoryMetadataStore::new();
        let metrics = MetricsRegistry::default();
        let mut daemon = Daemon::with_metrics(store, metrics.clone());

        metrics.record_job_spawned(JobKind::Prefetch);
        metrics.record_job_failed(JobKind::Prefetch);
        metrics.record_job_completed(JobKind::Maintenance);

        match daemon.handle(DaemonCommand::Metrics) {
            DaemonResponse::Metrics(snapshot) => {
                let prefetch = snapshot.jobs.get(&JobKind::Prefetch).unwrap();
                assert_eq!(prefetch.spawned, 1);
                assert_eq!(prefetch.failed, 1);
                assert_eq!(prefetch.completed, 0);
                let maintenance = snapshot.jobs.get(&JobKind::Maintenance).unwrap();
                assert_eq!(maintenance.completed, 1);
                assert!(!snapshot.repos.is_empty() || snapshot.global.pending_jobs == 0);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn fsmonitor_snapshot_returns_dirty_paths() {
        let store = InMemoryMetadataStore::new();
        let mut daemon = Daemon::new(store);
        let (_repo_dir, repo) = init_temp_repo();
        match daemon.handle(DaemonCommand::RegisterRepo {
            repo_path: repo.clone(),
        }) {
            DaemonResponse::Ack(_) => {}
            other => panic!("unexpected response: {other:?}"),
        }
        daemon.handle_watch_event(
            &repo,
            &WatchEvent::new(repo.join("file.txt"), WatchEventKind::Modified),
        );
        let response = daemon.handle(DaemonCommand::FsMonitorSnapshot {
            repo_path: repo.clone(),
            last_seen_generation: None,
        });
        match response {
            DaemonResponse::FsMonitorSnapshot(snapshot) => {
                assert_eq!(snapshot.repo_path, repo);
                assert!(snapshot.dirty_paths.contains(&PathBuf::from("file.txt")));
                assert!(snapshot.generation > 0);
            }
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn runtime_processes_jobs() {
        let runtime = Runtime::new(InMemoryMetadataStore::new(), None);
        let shutdown = runtime.shutdown_signal();
        let shared = runtime.shared();
        let runtime_task = tokio::spawn(runtime.run());

        let service = InProcessService::from_shared(shared.clone());
        let (_repo_dir, repo) = init_temp_repo();
        service
            .execute(DaemonCommand::RegisterRepo {
                repo_path: repo.clone(),
            })
            .await
            .unwrap();
        service
            .execute(DaemonCommand::QueueJob {
                repo_path: repo.clone(),
                job: JobKind::Maintenance,
            })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(300)).await;
        shutdown.shutdown();
        runtime_task.await.unwrap();

        match service.execute(DaemonCommand::ListRepos).await.unwrap() {
            DaemonResponse::RepoList(entries) => assert_eq!(entries[0].pending_jobs, 0),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[tokio::test]
    async fn nng_client_server_roundtrip() {
        let runtime = Runtime::new(InMemoryMetadataStore::new(), None);
        let shutdown = runtime.shutdown_signal();
        let shared = runtime.shared();
        let address = test_address();
        let server = NngServer::new(address.clone(), shared, shutdown.clone());
        let runtime_task = tokio::spawn(runtime.run());
        let server_task = tokio::spawn(async move {
            server.run().await.expect("server exits cleanly");
        });

        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = NngClient::new(address);
        let (_repo_dir, repo_path) = init_temp_repo();
        client
            .execute(DaemonCommand::RegisterRepo {
                repo_path: repo_path.clone(),
            })
            .await
            .unwrap();

        match client.execute(DaemonCommand::ListRepos).await.unwrap() {
            DaemonResponse::RepoList(entries) => assert_eq!(entries.len(), 1),
            other => panic!("unexpected response: {other:?}"),
        }

        shutdown.shutdown();
        runtime_task.await.unwrap();
        server_task.await.unwrap();
    }

    #[tokio::test]
    async fn watcher_events_record_last_event() {
        let runtime = Runtime::with_watcher(
            InMemoryMetadataStore::new(),
            Arc::new(ManualWatcher::new()),
            None,
        );
        let watcher_state = runtime.watcher_state();
        let shutdown = runtime.shutdown_signal();
        let shared = runtime.shared();
        let runtime_task = tokio::spawn(runtime.run());

        let service = InProcessService::from_shared(shared);
        let (_dir, repo_path) = init_temp_repo();
        std::fs::write(repo_path.join("file.txt"), "data").expect("write file");
        service
            .execute(DaemonCommand::RegisterRepo {
                repo_path: repo_path.clone(),
            })
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(300)).await;

        let manual_handle = {
            let state = watcher_state.lock().expect("watcher state poisoned");
            let registration = state.values().next().expect("watcher registered");
            registration.handle.clone()
        };
        let manual = manual_handle
            .as_any()
            .downcast_ref::<ManualWatchHandle>()
            .expect("manual handle");
        manual
            .emit(WatchEvent::new(
                repo_path.join("file.txt"),
                WatchEventKind::Modified,
            ))
            .unwrap();

        tokio::time::sleep(Duration::from_millis(300)).await;

        match service.execute(DaemonCommand::ListRepos).await.unwrap() {
            DaemonResponse::RepoList(entries) => {
                assert!(entries[0].last_event.is_some());
            }
            other => panic!("unexpected response: {other:?}"),
        }

        let status_response = service
            .execute(DaemonCommand::Status {
                repo_path: repo_path.clone(),
                known_generation: None,
            })
            .await
            .unwrap();
        match status_response {
            DaemonResponse::RepoStatus(detail) => {
                assert!(
                    detail.dirty_paths.contains(&PathBuf::from("file.txt")),
                    "status should include modified file: {:?}",
                    detail.dirty_paths
                );
                assert!(detail.generation > 0);
            }
            other => panic!("unexpected response: {other:?}"),
        }

        shutdown.shutdown();
        runtime_task.await.unwrap();
    }

    fn init_temp_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempdir().expect("create temp dir");
        Repository::init(dir.path()).expect("init repo");
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    fn test_address() -> String {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let addr = listener.local_addr().unwrap();
        drop(listener);
        format!("tcp://{}", addr)
    }
}
