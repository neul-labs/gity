use async_trait::async_trait;
use notify::{
    Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher as NotifyWatcherTrait,
};
use std::{
    any::Any,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::sync::mpsc;

pub type WatchReceiver = mpsc::UnboundedReceiver<WatchEvent>;
pub type WatchSender = mpsc::UnboundedSender<WatchEvent>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchEvent {
    pub path: PathBuf,
    pub kind: WatchEventKind,
}

impl WatchEvent {
    pub fn new(path: PathBuf, kind: WatchEventKind) -> Self {
        Self { path, kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchEventKind {
    Created,
    Modified,
    Deleted,
}

#[derive(Error, Debug)]
pub enum WatchError {
    #[error("watcher backend error: {0}")]
    Backend(String),
}

impl From<std::io::Error> for WatchError {
    fn from(value: std::io::Error) -> Self {
        Self::Backend(value.to_string())
    }
}

impl From<mpsc::error::SendError<WatchEvent>> for WatchError {
    fn from(value: mpsc::error::SendError<WatchEvent>) -> Self {
        Self::Backend(value.to_string())
    }
}

impl From<notify::Error> for WatchError {
    fn from(value: notify::Error) -> Self {
        Self::Backend(value.to_string())
    }
}

pub type WatchHandleRef = Arc<dyn WatchHandle>;

pub struct WatchSubscription {
    handle: WatchHandleRef,
    receiver: WatchReceiver,
}

impl WatchSubscription {
    pub fn new(handle: WatchHandleRef, receiver: WatchReceiver) -> Self {
        Self { handle, receiver }
    }

    pub fn handle(&self) -> WatchHandleRef {
        Arc::clone(&self.handle)
    }

    pub fn into_parts(self) -> (WatchHandleRef, WatchReceiver) {
        (self.handle, self.receiver)
    }
}

pub trait WatchHandle: Send + Sync {
    fn stop(&self);
    fn as_any(&self) -> &dyn Any;
}

#[async_trait]
pub trait Watcher: Send + Sync {
    async fn watch(&self, repo_path: PathBuf) -> Result<WatchSubscription, WatchError>;
}

pub type WatcherRef = Arc<dyn Watcher>;

/// Manual watcher used in tests to emit events manually.
#[derive(Debug, Default, Clone, Copy)]
pub struct ManualWatcher;

#[derive(Debug)]
pub struct ManualWatchHandle {
    repo_path: PathBuf,
    sender: Mutex<Option<WatchSender>>,
}

impl ManualWatcher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Watcher for ManualWatcher {
    async fn watch(&self, repo_path: PathBuf) -> Result<WatchSubscription, WatchError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let handle = Arc::new(ManualWatchHandle {
            repo_path,
            sender: Mutex::new(Some(sender)),
        });
        let handle_ref: WatchHandleRef = handle;
        Ok(WatchSubscription::new(handle_ref, receiver))
    }
}

impl ManualWatchHandle {
    pub fn emit(&self, event: WatchEvent) -> Result<(), WatchError> {
        let sender = self
            .sender
            .lock()
            .map_err(|_| WatchError::Backend("watch handle poisoned".into()))?;
        match sender.as_ref() {
            Some(tx) => tx.send(event).map_err(WatchError::from),
            None => Err(WatchError::Backend("watcher stopped".into())),
        }
    }

    pub fn emit_path(
        &self,
        kind: WatchEventKind,
        path: impl Into<PathBuf>,
    ) -> Result<(), WatchError> {
        self.emit(WatchEvent::new(path.into(), kind))
    }

    pub fn repo_path(&self) -> &Path {
        &self.repo_path
    }
}

impl WatchHandle for ManualWatchHandle {
    fn stop(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Watcher that never emits events; used as a placeholder until platform
/// backends are wired up.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopWatcher;

#[derive(Debug)]
pub struct NoopWatchHandle {
    sender: Mutex<Option<WatchSender>>,
}

impl NoopWatcher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Watcher for NoopWatcher {
    async fn watch(&self, _repo_path: PathBuf) -> Result<WatchSubscription, WatchError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let handle = Arc::new(NoopWatchHandle {
            sender: Mutex::new(Some(sender)),
        });
        let handle_ref: WatchHandleRef = handle;
        Ok(WatchSubscription::new(handle_ref, receiver))
    }
}

impl WatchHandle for NoopWatchHandle {
    fn stop(&self) {
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Watcher backed by the `notify` crate for real filesystem monitoring.
#[derive(Debug, Default, Clone, Copy)]
pub struct NotifyWatcher;

#[derive(Debug)]
pub struct NotifyWatchHandle {
    watcher: Mutex<Option<RecommendedWatcher>>,
    sender: Mutex<Option<WatchSender>>,
}

impl NotifyWatcher {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Watcher for NotifyWatcher {
    async fn watch(&self, repo_path: PathBuf) -> Result<WatchSubscription, WatchError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        let closure_sender = sender.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Err(err) = res.map(|event| dispatch_event(&closure_sender, event)) {
                    eprintln!("notify error: {err}");
                }
            },
            Config::default(),
        )
        .map_err(WatchError::from)?;
        watcher
            .watch(&repo_path, RecursiveMode::Recursive)
            .map_err(WatchError::from)?;
        let handle = Arc::new(NotifyWatchHandle::new(watcher, sender));
        let handle_ref: WatchHandleRef = handle;
        Ok(WatchSubscription::new(handle_ref, receiver))
    }
}

impl NotifyWatchHandle {
    fn new(watcher: RecommendedWatcher, sender: WatchSender) -> Self {
        Self {
            watcher: Mutex::new(Some(watcher)),
            sender: Mutex::new(Some(sender)),
        }
    }
}

impl WatchHandle for NotifyWatchHandle {
    fn stop(&self) {
        if let Ok(mut watcher) = self.watcher.lock() {
            watcher.take();
        }
        if let Ok(mut sender) = self.sender.lock() {
            sender.take();
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn dispatch_event(sender: &WatchSender, event: Event) -> Result<(), WatchError> {
    if let Some(kind) = map_event_kind(&event.kind) {
        for path in event.paths {
            sender.send(WatchEvent::new(path, kind.clone()))?;
        }
    }
    Ok(())
}

fn map_event_kind(kind: &EventKind) -> Option<WatchEventKind> {
    use notify::event::{CreateKind, ModifyKind, RemoveKind};
    match kind {
        EventKind::Create(CreateKind::Any) | EventKind::Create(_) => Some(WatchEventKind::Created),
        EventKind::Modify(ModifyKind::Data(_))
        | EventKind::Modify(ModifyKind::Metadata(_))
        | EventKind::Modify(ModifyKind::Any)
        | EventKind::Modify(_) => Some(WatchEventKind::Modified),
        EventKind::Remove(RemoveKind::Any) | EventKind::Remove(_) => Some(WatchEventKind::Deleted),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path, time::Duration};
    use tempfile::tempdir;
    use tokio::{runtime::Runtime, time::timeout};

    #[test]
    fn manual_watcher_emits_events() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let watcher = ManualWatcher::new();
            let subscription = watcher
                .watch(PathBuf::from("/tmp/manual"))
                .await
                .expect("start watcher");
            let (handle, mut receiver) = subscription.into_parts();
            let manual = handle
                .as_any()
                .downcast_ref::<ManualWatchHandle>()
                .expect("manual handle");
            assert_eq!(manual.repo_path(), Path::new("/tmp/manual"));
            manual
                .emit_path(WatchEventKind::Created, "/tmp/manual/foo.txt")
                .expect("emit event");
            let event = receiver.recv().await.expect("receive event");
            assert_eq!(event.kind, WatchEventKind::Created);
            assert_eq!(event.path, PathBuf::from("/tmp/manual/foo.txt"));
        });
    }

    #[test]
    fn manual_handle_stop_closes_stream() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let watcher = ManualWatcher::new();
            let subscription = watcher
                .watch(PathBuf::from("/tmp/manual"))
                .await
                .expect("start watcher");
            let (handle, mut receiver) = subscription.into_parts();
            handle.stop();
            assert!(receiver.recv().await.is_none());
        });
    }

    #[test]
    fn noop_watcher_stop_closes_receiver() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let watcher = NoopWatcher::new();
            let subscription = watcher
                .watch(PathBuf::from("/tmp/noop"))
                .await
                .expect("start watcher");
            let (handle, mut receiver) = subscription.into_parts();
            handle.stop();
            assert!(receiver.recv().await.is_none());
        });
    }

    #[test]
    fn notify_watcher_emits_real_events() {
        let runtime = Runtime::new().expect("runtime");
        runtime.block_on(async {
            let dir = tempdir().expect("temp dir");
            // Canonicalize to handle macOS FSEvents symlink resolution
            let canonical_dir =
                std::fs::canonicalize(dir.path()).unwrap_or_else(|_| dir.path().to_path_buf());
            let watcher = NotifyWatcher::new();
            let subscription = watcher
                .watch(canonical_dir.clone())
                .await
                .expect("start watcher");
            let (handle, mut receiver) = subscription.into_parts();
            let file_path = canonical_dir.join("notify.txt");
            fs::write(&file_path, "data").expect("write file");
            let event = timeout(Duration::from_secs(2), receiver.recv())
                .await
                .expect("watch timed out")
                .expect("event");
            assert!(event.path.ends_with(Path::new("notify.txt")));
            handle.stop();
        });
    }
}
