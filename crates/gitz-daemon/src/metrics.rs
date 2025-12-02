use gitz_ipc::{DaemonMetrics, GlobalMetrics, JobKind, JobMetrics};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use sysinfo::{Pid, System};
use tracing::warn;

#[derive(Clone)]
pub struct MetricsRegistry {
    state: Arc<Mutex<MetricsState>>,
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(MetricsState::default())),
        }
    }
}

impl MetricsRegistry {
    pub fn record_job_spawned(&self, kind: JobKind) {
        self.update(kind, |metrics| metrics.spawned += 1);
    }

    pub fn record_job_completed(&self, kind: JobKind) {
        self.update(kind, |metrics| metrics.completed += 1);
    }

    pub fn record_job_failed(&self, kind: JobKind) {
        self.update(kind, |metrics| metrics.failed += 1);
    }

    pub fn snapshot(&self) -> DaemonMetrics {
        match self.state.lock() {
            Ok(state) => DaemonMetrics {
                jobs: state.jobs.clone(),
                global: GlobalMetrics::default(),
                repos: Vec::new(),
            },
            Err(err) => {
                warn!("metrics state poisoned: {err}");
                DaemonMetrics::new()
            }
        }
    }

    fn update(&self, kind: JobKind, mutate: impl FnOnce(&mut JobMetrics)) {
        match self.state.lock() {
            Ok(mut state) => {
                let counters = state.jobs.entry(kind).or_default();
                mutate(counters);
            }
            Err(err) => {
                warn!("metrics state poisoned: {err}");
            }
        }
    }
}

#[derive(Default)]
struct MetricsState {
    jobs: HashMap<JobKind, JobMetrics>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ResourceSnapshot {
    pub cpu_percent: f32,
    pub rss_bytes: u64,
}

pub fn collect_resource_snapshot() -> ResourceSnapshot {
    let pid = Pid::from_u32(std::process::id());
    let mut system = System::new_all();
    system.refresh_process(pid);
    if let Some(process) = system.process(pid) {
        ResourceSnapshot {
            cpu_percent: process.cpu_usage(),
            rss_bytes: process.memory() * 1024,
        }
    } else {
        ResourceSnapshot::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_increment_and_snapshot() {
        let metrics = MetricsRegistry::default();
        metrics.record_job_spawned(JobKind::Prefetch);
        metrics.record_job_completed(JobKind::Prefetch);
        metrics.record_job_failed(JobKind::Prefetch);
        metrics.record_job_spawned(JobKind::Maintenance);

        let snapshot = metrics.snapshot();
        let prefetch = snapshot.jobs.get(&JobKind::Prefetch).unwrap();
        assert_eq!(prefetch.spawned, 1);
        assert_eq!(prefetch.completed, 1);
        assert_eq!(prefetch.failed, 1);

        let maintenance = snapshot.jobs.get(&JobKind::Maintenance).unwrap();
        assert_eq!(maintenance.spawned, 1);
        assert_eq!(maintenance.completed, 0);
        assert_eq!(maintenance.failed, 0);
    }
}
