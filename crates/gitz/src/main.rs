mod daemon_launcher;
mod paths;
mod status_cache;

use clap::Parser;
use gitz_cli::{format_repo_status, format_response, Cli, CliAction, CliError};
use gitz_daemon::{
    NotificationServer, NotificationSubscriber, NngClient, NngServer, Runtime, ServerError,
};
use gitz_ipc::{
    DaemonCommand, DaemonError, DaemonNotification, DaemonResponse, DaemonService, DaemonHealth,
    JobEventKind, RepoStatusDetail, WatchEventKind,
};
use gitz_storage::{StorageContext, StorageError};
use std::{
    env,
    fs::OpenOptions,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    pin, signal,
    sync::mpsc,
    time::{Duration, sleep},
};

use crate::{daemon_launcher::spawn_daemon, paths::GitzPaths, status_cache::StatusCache};

const DEFAULT_ADDR: &str = "tcp://127.0.0.1:7557";
const DEFAULT_EVENTS_ADDR: &str = "tcp://127.0.0.1:7558";

#[tokio::main]
async fn main() {
    if let Err(err) = try_main().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn try_main() -> Result<(), CliError> {
    let cli = Cli::parse();
    let address = default_address();
    let events_address = default_events_address();
    match cli.into_action() {
        CliAction::RunDaemon => run_daemon(address, events_address).await,
        action => run_client_action(action, &address, &events_address).await,
    }
}

async fn run_client_action(
    action: CliAction,
    address: &str,
    events_address: &str,
) -> Result<(), CliError> {
    match action {
        CliAction::Rpc(command) => run_command_with_auto_start(command, address).await,
        CliAction::List { stats } => run_list_command(address, stats).await,
        CliAction::Logs {
            repo_path,
            follow,
            limit,
        } => run_logs_command(address, repo_path, follow, limit).await,
        CliAction::FsMonitorHelper {
            version,
            token,
            repo,
        } => run_fsmonitor_helper(address, version, token, repo).await,
        CliAction::StreamEvents => {
            ensure_daemon_running(address).await?;
            run_event_stream(events_address).await
        }
        CliAction::RunDaemon => unreachable!("handled earlier"),
        CliAction::StartDaemon => run_start_daemon(address).await,
        CliAction::StopDaemon => run_stop_daemon(address).await,
        CliAction::OneshotDaemon { repo_path } => {
            run_oneshot_daemon(address, events_address, repo_path).await
        }
        CliAction::RunTray => run_tray(address).await,
    }
}

#[cfg(feature = "tray")]
async fn run_tray(address: &str) -> Result<(), CliError> {
    use gitz_tray::{GitzTray, TrayConfig, run_tray_loop};

    // Make sure daemon is running first
    ensure_daemon_running(address).await?;

    let config = TrayConfig {
        daemon_address: address.to_string(),
    };

    let tray = GitzTray::new(config)
        .map_err(|e| CliError::Message(format!("failed to create tray: {}", e)))?;

    println!("System tray started. Right-click the icon for options.");
    run_tray_loop(&tray);

    Ok(())
}

#[cfg(not(feature = "tray"))]
async fn run_tray(_address: &str) -> Result<(), CliError> {
    Err(CliError::Message(
        "tray feature not enabled - rebuild with --features tray".into(),
    ))
}

async fn run_command_with_auto_start(
    command: DaemonCommand,
    address: &str,
) -> Result<(), CliError> {
    let client = NngClient::new(address.to_string());
    match command {
        DaemonCommand::Status { repo_path, .. } => {
            match run_status_command(&client, repo_path.clone(), address).await {
                Err(CliError::Daemon(DaemonError::Transport(_))) => {
                    ensure_daemon_running(address).await?;
                    run_status_command(&client, repo_path, address).await
                }
                result => result,
            }
        }
        other => run_generic_command(&client, other, address).await,
    }
}

async fn run_generic_command(
    client: &NngClient,
    command: DaemonCommand,
    address: &str,
) -> Result<(), CliError> {
    let response = request_with_restart(client, address, command).await?;
    println!("{}", format_response(&response));
    Ok(())
}

async fn run_list_command(address: &str, stats: bool) -> Result<(), CliError> {
    let client = NngClient::new(address.to_string());
    run_generic_command(&client, DaemonCommand::ListRepos, address).await?;
    if stats {
        let metrics = request_with_restart(&client, address, DaemonCommand::Metrics).await?;
        println!("{}", format_response(&metrics));
    }
    Ok(())
}

async fn run_fsmonitor_helper(
    address: &str,
    version: u8,
    token: Option<String>,
    repo_override: Option<PathBuf>,
) -> Result<(), CliError> {
    if version != 2 {
        return Err(CliError::Message(format!(
            "unsupported fsmonitor protocol version {version}"
        )));
    }
    let repo_path = resolve_repo_path(repo_override)?;
    let known_generation = token
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok());
    let client = NngClient::new(address.to_string());
    let response = request_with_restart(
        &client,
        address,
        DaemonCommand::FsMonitorSnapshot {
            repo_path: repo_path.clone(),
            last_seen_generation: known_generation,
        },
    )
    .await?;
    let snapshot = match response {
        DaemonResponse::FsMonitorSnapshot(snapshot) => snapshot,
        other => {
            return Err(CliError::Message(format!(
                "unexpected fsmonitor response: {other:?}"
            )));
        }
    };
    emit_fsmonitor_payload(version, &snapshot).map_err(map_io_error)
}

async fn run_logs_command(
    address: &str,
    repo_path: PathBuf,
    follow: bool,
    limit: usize,
) -> Result<(), CliError> {
    let client = NngClient::new(address.to_string());
    let response = request_with_restart(
        &client,
        address,
        DaemonCommand::FetchLogs {
            repo_path: repo_path.clone(),
            limit,
        },
    )
    .await?;
    match response {
        DaemonResponse::Logs(entries) => {
            if entries.is_empty() {
                println!("no log entries for {}", repo_path.display());
            } else {
                for entry in entries {
                    print_log_entry(&entry);
                }
            }
        }
        other => {
            println!("{}", format_response(&other));
        }
    }
    if follow {
        ensure_daemon_running(address).await?;
        follow_log_stream(address, repo_path).await?;
    }
    Ok(())
}

async fn run_event_stream(address: &str) -> Result<(), CliError> {
    let subscriber = NotificationSubscriber::new(address.to_string());
    let mut stream = subscriber.connect().await.map_err(CliError::Daemon)?;
    println!(
        "listening for daemon notifications on {address} (Ctrl+C to exit)..."
    );
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            notification = stream.next() => match notification {
                Ok(notification) => print_notification(&notification),
                Err(err) => return Err(CliError::Daemon(err)),
            }
        }
    }
    Ok(())
}

async fn follow_log_stream(address: &str, repo_path: PathBuf) -> Result<(), CliError> {
    let subscriber = NotificationSubscriber::new(address.to_string());
    let mut stream = subscriber.connect().await.map_err(CliError::Daemon)?;
    println!("following logs for {} (Ctrl+C to exit)...", repo_path.display());
    loop {
        tokio::select! {
            _ = signal::ctrl_c() => break,
            notification = stream.next() => match notification {
                Ok(DaemonNotification::Log(entry)) => {
                    if entry.repo_path == repo_path {
                        print_log_entry(&entry);
                    }
                }
                Ok(_) => {}
                Err(err) => return Err(CliError::Daemon(err)),
            }
        }
    }
    Ok(())
}

async fn run_start_daemon(address: &str) -> Result<(), CliError> {
    let client = NngClient::new(address.to_string());
    match client.execute(DaemonCommand::HealthCheck).await {
        Ok(_) => {
            println!("daemon already running on {address}");
            return Ok(());
        }
        Err(_) => {}
    }
    spawn_daemon(address)
        .map_err(|err| CliError::Message(format!("failed to start daemon: {err}")))?;
    let mut attempts = 0;
    let max_attempts = 25;
    while attempts < max_attempts {
        match client.execute(DaemonCommand::HealthCheck).await {
            Ok(_) => {
                println!("daemon started on {address}");
                return Ok(());
            }
            Err(_) => {
                attempts += 1;
                sleep(Duration::from_millis(200)).await;
            }
        }
    }
    Err(CliError::Message("timed out waiting for daemon to start".into()))
}

async fn run_stop_daemon(address: &str) -> Result<(), CliError> {
    let client = NngClient::new(address.to_string());
    match client.execute(DaemonCommand::Shutdown).await {
        Ok(response) => {
            println!("{}", format_response(&response));
            Ok(())
        }
        Err(DaemonError::Transport(_)) => {
            println!("daemon not running");
            Ok(())
        }
        Err(err) => Err(CliError::Daemon(err)),
    }
}

async fn run_oneshot_daemon(
    address: &str,
    events_address: &str,
    repo_path: PathBuf,
) -> Result<(), CliError> {
    let paths = GitzPaths::discover().map_err(map_io_error)?;
    let storage = StorageContext::new(paths.data_dir()).map_err(map_storage_error)?;
    let store = storage.metadata_store().map_err(map_storage_error)?;
    let log_tree = storage.log_tree().map_err(map_storage_error)?;
    let helper_command = fsmonitor_helper_command();

    let (notification_tx, notification_rx) = mpsc::unbounded_channel();
    let runtime = Runtime::with_notifications(
        store,
        Some(notification_tx),
        helper_command,
        Some(log_tree),
    );
    let shutdown = runtime.shutdown_signal();
    let shared = runtime.shared();
    let service = runtime.service_handle();

    // Register the repo if not already registered
    let _ = service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: repo_path.clone(),
        })
        .await;

    let runtime_fut = runtime.run();
    let server = NngServer::new(address.to_string(), shared.clone(), shutdown.clone());
    let server_fut = server.run();
    let notification_server = NotificationServer::new(events_address.to_string(), notification_rx);
    let notification_fut = notification_server.run(shutdown.clone());

    pin!(runtime_fut);
    pin!(server_fut);
    pin!(notification_fut);

    println!("oneshot daemon for {} on {address}", repo_path.display());

    // Run until jobs are complete or timeout
    let timeout = sleep(Duration::from_secs(30));
    pin!(timeout);

    loop {
        let pending_jobs = match service.execute(DaemonCommand::HealthCheck).await {
            Ok(DaemonResponse::Health(health)) => health.pending_jobs,
            _ => 0,
        };

        if pending_jobs == 0 {
            println!("all jobs complete, shutting down");
            break;
        }

        tokio::select! {
            _ = &mut timeout => {
                println!("timeout reached, shutting down");
                break;
            }
            _ = sleep(Duration::from_millis(500)) => continue,
        }
    }

    shutdown.shutdown();
    let _ = runtime_fut.await;
    let _ = server_fut.await;
    let _ = notification_fut.await;

    Ok(())
}

async fn run_status_command(
    client: &NngClient,
    repo_path: PathBuf,
    address: &str,
) -> Result<(), CliError> {
    let paths = GitzPaths::discover().map_err(map_io_error)?;
    let cache = StatusCache::new(paths.data_dir()).map_err(map_io_error)?;
    let cached = cache.load(&repo_path).map_err(map_io_error)?;
    let cached_detail = cached.clone();
    let known_generation = cached.as_ref().map(|detail| detail.generation);
    let response = request_with_restart(
        client,
        address,
        DaemonCommand::Status {
            repo_path: repo_path.clone(),
            known_generation,
        },
    )
    .await?;
    let decision = resolve_status_decision(response, cached_detail);
    if let Some(detail) = decision.to_cache.as_ref() {
        cache.store(detail).map_err(map_io_error)?;
    }
    if let Some(err) = decision.stderr {
        eprintln!("{err}");
    }
    println!("{}", decision.stdout);
    Ok(())
}

fn print_notification(notification: &DaemonNotification) {
    match notification {
        DaemonNotification::WatchEvent(event) => println!(
            "[watch] {} -> {} ({})",
            event.repo_path.display(),
            event.path.display(),
            format_watch_kind(event.kind)
        ),
        DaemonNotification::JobEvent(event) => println!(
            "[job] {:?} for {} ({})",
            event.job,
            event.repo_path.display(),
            format_job_event_kind(event.kind)
        ),
        DaemonNotification::RepoStatus(detail) => println!(
            "[status] {} dirty paths={}, generation={}",
            detail.repo_path.display(),
            detail.dirty_paths.len(),
            detail.generation
        ),
        DaemonNotification::Log(entry) => print_log_entry(entry),
    }
}

fn print_log_entry(entry: &gitz_ipc::LogEntry) {
    let timestamp = entry
        .timestamp
        .duration_since(UNIX_EPOCH)
        .map(|dur| dur.as_secs())
        .unwrap_or_default();
    println!(
        "[log {}] {}: {}",
        timestamp,
        entry.repo_path.display(),
        entry.message
    );
}

fn format_watch_kind(kind: WatchEventKind) -> &'static str {
    match kind {
        WatchEventKind::Created => "created",
        WatchEventKind::Modified => "modified",
        WatchEventKind::Deleted => "deleted",
    }
}

fn format_job_event_kind(kind: JobEventKind) -> &'static str {
    match kind {
        JobEventKind::Queued => "queued",
        JobEventKind::Started => "started",
        JobEventKind::Completed => "completed",
        JobEventKind::Failed => "failed",
    }
}

struct StatusDecision {
    stdout: String,
    stderr: Option<String>,
    to_cache: Option<RepoStatusDetail>,
}

fn resolve_status_decision(
    response: DaemonResponse,
    cached: Option<RepoStatusDetail>,
) -> StatusDecision {
    match response {
        DaemonResponse::RepoStatus(detail) => StatusDecision {
            stdout: format_repo_status(&detail),
            stderr: None,
            to_cache: Some(detail),
        },
        DaemonResponse::RepoStatusUnchanged {
            repo_path,
            generation,
        } => {
            if let Some(detail) = cached {
                StatusDecision {
                    stdout: format_repo_status(&detail),
                    stderr: None,
                    to_cache: None,
                }
            } else {
                StatusDecision {
                    stdout: format!(
                        "{}",
                        format_response(&DaemonResponse::RepoStatusUnchanged { repo_path, generation })
                    ),
                    stderr: None,
                    to_cache: None,
                }
            }
        }
        DaemonResponse::Error(message) => {
            if let Some(detail) = cached {
                StatusDecision {
                    stdout: format_repo_status(&detail),
                    stderr: Some(format!("daemon error: {message}; showing cached status")),
                    to_cache: None,
                }
            } else {
                StatusDecision {
                    stdout: message,
                    stderr: None,
                    to_cache: None,
                }
            }
        }
        other => StatusDecision {
            stdout: format_response(&other),
            stderr: None,
            to_cache: None,
        },
    }
}

async fn request_with_restart(
    client: &NngClient,
    address: &str,
    command: DaemonCommand,
) -> Result<DaemonResponse, CliError> {
    match client.execute(command.clone()).await {
        Ok(response) => Ok(response),
        Err(DaemonError::Transport(_)) => {
            ensure_daemon_running(address).await?;
            client.execute(command).await.map_err(CliError::Daemon)
        }
        Err(err) => Err(CliError::Daemon(err)),
    }
}

async fn run_daemon(address: String, events_address: String) -> Result<(), CliError> {
    let paths = GitzPaths::discover().map_err(map_io_error)?;
    let storage = StorageContext::new(paths.data_dir()).map_err(map_storage_error)?;
    append_daemon_log(
        paths.daemon_log_path(),
        &format!("Starting daemon on {address}"),
    )?;
    let store = storage.metadata_store().map_err(map_storage_error)?;

    let helper_command = fsmonitor_helper_command();
    let (notification_tx, notification_rx) = mpsc::unbounded_channel();
    let log_tree = storage.log_tree().map_err(map_storage_error)?;
    let runtime = Runtime::with_notifications(
        store,
        Some(notification_tx),
        helper_command.clone(),
        Some(log_tree),
    );
    let shutdown = runtime.shutdown_signal();
    let shared = runtime.shared();
    let runtime_fut = runtime.run();
    let server = NngServer::new(address.clone(), shared, shutdown.clone());
    let server_fut = server.run();
    let notification_server =
        NotificationServer::new(events_address.clone(), notification_rx);
    let notification_fut = notification_server.run(shutdown.clone());

    pin!(runtime_fut);
    pin!(server_fut);
    pin!(notification_fut);

    println!("Daemon listening on {address}");

    let mut server_result: Option<Result<(), ServerError>> = None;
    let mut notification_result: Option<Result<(), ServerError>> = None;
    tokio::select! {
        _ = &mut runtime_fut => (),
        result = &mut server_fut => {
            server_result = Some(result);
        }
        result = &mut notification_fut => {
            notification_result = Some(result);
        }
        _ = signal::ctrl_c() => {
            println!("Ctrl+C received, shutting down...");
        }
    }

    shutdown.shutdown();
    let _ = runtime_fut.await;
    if server_result.is_none() {
        server_result = Some(server_fut.await);
    }
    if notification_result.is_none() {
        notification_result = Some(notification_fut.await);
    }

    if let Some(Err(err)) = server_result {
        return Err(CliError::Message(format!("daemon server error: {err}")));
    }
    if let Some(Err(err)) = notification_result {
        return Err(CliError::Message(format!("notification server error: {err}")));
    }
    append_daemon_log(paths.daemon_log_path(), "Daemon shutdown complete")?;
    Ok(())
}

fn default_address() -> String {
    std::env::var("GITZ_DAEMON_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string())
}

fn default_events_address() -> String {
    std::env::var("GITZ_EVENTS_ADDR").unwrap_or_else(|_| DEFAULT_EVENTS_ADDR.to_string())
}

async fn ensure_daemon_running(address: &str) -> Result<(), CliError> {
    let paths = GitzPaths::discover().map_err(map_io_error)?;
    let _ = StorageContext::new(paths.data_dir()).map_err(map_storage_error)?;
    spawn_daemon(address)
        .map_err(|err| CliError::Message(format!("failed to start daemon: {err}")))?;
    let client = NngClient::new(address.to_string());
    let mut attempts = 0;
    let max_attempts = 25;
    while attempts < max_attempts {
        match client.execute(DaemonCommand::HealthCheck).await {
            Ok(_) => return Ok(()),
            Err(DaemonError::Transport(_)) => {
                attempts += 1;
                sleep(Duration::from_millis(200)).await;
            }
            Err(err) => return Err(CliError::Daemon(err)),
        }
    }
    Err(CliError::Message(
        "timed out waiting for daemon to start".into(),
    ))
}

fn append_daemon_log(path: impl AsRef<Path>, message: &str) -> Result<(), CliError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_ref())
        .map_err(map_io_error)?;
    writeln!(file, "[{timestamp}] {message}").map_err(map_io_error)?;
    Ok(())
}

fn map_io_error(err: io::Error) -> CliError {
    CliError::Message(err.to_string())
}

fn map_storage_error(err: StorageError) -> CliError {
    CliError::Message(err.to_string())
}

fn fsmonitor_helper_command() -> Option<String> {
    if let Ok(value) = env::var("GITZ_FSMONITOR_HELPER") {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return None;
        }
        return Some(value);
    }
    env::current_exe()
        .ok()
        .map(|path| format!("\"{}\" fsmonitor-helper", path.display()))
}

fn resolve_repo_path(repo_override: Option<PathBuf>) -> Result<PathBuf, CliError> {
    if let Some(path) = repo_override {
        return Ok(path);
    }
    if let Ok(git_dir) = env::var("GIT_DIR") {
        let cwd = env::current_dir().map_err(map_io_error)?;
        let candidate = if Path::new(&git_dir).is_absolute() {
            PathBuf::from(git_dir)
        } else {
            cwd.join(git_dir)
        };
        if candidate.file_name().and_then(|name| name.to_str()) == Some(".git") {
            return candidate
                .parent()
                .map(|parent| parent.to_path_buf())
                .ok_or_else(|| CliError::Message("unable to resolve repo root".into()));
        } else {
            return Ok(candidate);
        }
    }
    env::current_dir().map_err(map_io_error)
}

fn emit_fsmonitor_payload(
    _version: u8,
    snapshot: &gitz_ipc::FsMonitorSnapshot,
) -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    stdout.write_all(snapshot.generation.to_string().as_bytes())?;
    stdout.write_all(b"\0")?;
    for path in &snapshot.dirty_paths {
        let path_str = path.to_string_lossy();
        stdout.write_all(path_str.as_bytes())?;
        stdout.write_all(b"\0")?;
    }
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_detail() -> RepoStatusDetail {
        RepoStatusDetail {
            repo_path: PathBuf::from("/repo"),
            dirty_paths: vec![PathBuf::from("file.txt")],
            generation: 5,
        }
    }

    #[test]
    fn resolve_status_decision_uses_cached_when_unchanged() {
        let cached = sample_detail();
        let response = DaemonResponse::RepoStatusUnchanged {
            repo_path: PathBuf::from("/repo"),
            generation: cached.generation,
        };
        let decision = resolve_status_decision(response, Some(cached.clone()));
        assert!(decision.to_cache.is_none());
        assert!(decision.stderr.is_none());
        assert_eq!(decision.stdout, format_repo_status(&cached));
    }

    #[test]
    fn resolve_status_decision_warns_and_uses_cache_on_error() {
        let cached = sample_detail();
        let response = DaemonResponse::Error("boom".into());
        let decision = resolve_status_decision(response, Some(cached.clone()));
        assert!(decision.to_cache.is_none());
        assert!(decision
            .stderr
            .as_deref()
            .is_some_and(|msg| msg.contains("showing cached status")));
        assert_eq!(decision.stdout, format_repo_status(&cached));
    }

    #[test]
    fn resolve_status_decision_surfaces_error_without_cache() {
        let response = DaemonResponse::Error("boom".into());
        let decision = resolve_status_decision(response, None);
        assert_eq!(decision.stdout, "boom");
        assert!(decision.stderr.is_none());
        assert!(decision.to_cache.is_none());
    }
}
