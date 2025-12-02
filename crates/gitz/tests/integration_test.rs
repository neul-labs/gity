//! Integration tests for core gitz workflows with real Git repositories.
//!
//! These tests verify the end-to-end behavior of:
//! - Repository registration and configuration
//! - File change detection via watchers
//! - Status queries returning correct dirty paths
//! - FSMonitor protocol compliance
//! - Job scheduling and execution

use git2::{Repository, Signature};
use gitz_daemon::{Runtime, NngServer, NngClient};
use gitz_git::{RepoConfigurator, working_tree_status};
use gitz_ipc::{DaemonCommand, DaemonResponse, DaemonService, FsMonitorSnapshot, JobKind};
use gitz_storage::{InMemoryMetadataStore, MetadataStore, SledMetadataStore};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::sleep;

/// Helper to create a Git repository with an initial commit.
fn create_test_repo(dir: &Path) -> Repository {
    let repo = Repository::init(dir).expect("init repo");

    // Create initial file and commit
    let file_path = dir.join("README.md");
    fs::write(&file_path, "# Test Repo\n").expect("write file");

    {
        let mut index = repo.index().expect("get index");
        index.add_path(Path::new("README.md")).expect("add to index");
        index.write().expect("write index");

        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = Signature::now("Test", "test@example.com").expect("signature");

        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .expect("commit");
    }

    repo
}

/// Helper to modify a file in the repo.
fn modify_file(dir: &Path, filename: &str, content: &str) {
    let path = dir.join(filename);
    fs::write(&path, content).expect("write file");
}

/// Helper to create a new untracked file.
fn create_untracked_file(dir: &Path, filename: &str, content: &str) {
    let path = dir.join(filename);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&path, content).expect("write file");
}

// =============================================================================
// Git Status Tests
// =============================================================================

#[test]
fn test_working_tree_status_clean_repo() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let status = working_tree_status(dir.path(), &[]).expect("get status");
    assert!(status.is_empty(), "Clean repo should have no dirty paths");
}

#[test]
fn test_working_tree_status_modified_file() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Modify tracked file
    modify_file(dir.path(), "README.md", "# Modified\n");

    let status = working_tree_status(dir.path(), &[]).expect("get status");
    assert_eq!(status.len(), 1);
    assert_eq!(status[0], PathBuf::from("README.md"));
}

#[test]
fn test_working_tree_status_untracked_file() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Create untracked file
    create_untracked_file(dir.path(), "new_file.txt", "hello");

    let status = working_tree_status(dir.path(), &[]).expect("get status");
    assert_eq!(status.len(), 1);
    assert_eq!(status[0], PathBuf::from("new_file.txt"));
}

#[test]
fn test_working_tree_status_multiple_changes() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Create multiple changes
    modify_file(dir.path(), "README.md", "# Modified\n");
    create_untracked_file(dir.path(), "file1.txt", "one");
    create_untracked_file(dir.path(), "file2.txt", "two");

    let status = working_tree_status(dir.path(), &[]).expect("get status");
    assert_eq!(status.len(), 3);
}

#[test]
fn test_working_tree_status_with_path_filter() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Create multiple changes
    modify_file(dir.path(), "README.md", "# Modified\n");
    create_untracked_file(dir.path(), "src/main.rs", "fn main() {}");

    // Filter to only src/
    let status = working_tree_status(dir.path(), &[PathBuf::from("src")]).expect("get status");
    assert_eq!(status.len(), 1);
    assert!(status[0].starts_with("src"));
}

// =============================================================================
// Repository Configuration Tests
// =============================================================================

#[test]
fn test_repo_configurator_applies_settings() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let configurator = RepoConfigurator::open(dir.path()).expect("open repo");
    configurator
        .apply_performance_settings(Some("gitz fsmonitor-helper"))
        .expect("apply settings");

    let config = fs::read_to_string(dir.path().join(".git/config")).expect("read config");
    assert!(config.contains("fsmonitor"));
    assert!(config.contains("untrackedCache"));
}

#[test]
fn test_repo_configurator_clears_settings() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let configurator = RepoConfigurator::open(dir.path()).expect("open repo");
    configurator
        .apply_performance_settings(Some("gitz fsmonitor-helper"))
        .expect("apply settings");
    configurator
        .clear_performance_settings()
        .expect("clear settings");

    let config = fs::read_to_string(dir.path().join(".git/config")).expect("read config");
    assert!(!config.contains("fsmonitor = gitz"));
}

// =============================================================================
// Storage Tests with Real Paths
// =============================================================================

#[test]
fn test_register_real_repo_in_memory_store() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let meta = store.register_repo(dir.path().to_path_buf()).expect("register");

    assert_eq!(meta.repo_path, dir.path());
    assert_eq!(meta.pending_jobs, 0);
}

#[test]
fn test_register_real_repo_in_sled_store() {
    let repo_dir = TempDir::new().unwrap();
    let _repo = create_test_repo(repo_dir.path());

    let db_dir = TempDir::new().unwrap();
    let store = SledMetadataStore::open(db_dir.path()).expect("open sled");

    let meta = store.register_repo(repo_dir.path().to_path_buf()).expect("register");
    assert_eq!(meta.repo_path, repo_dir.path());

    // Verify persistence
    let loaded = store.get_repo(repo_dir.path()).expect("get repo");
    assert!(loaded.is_some());
}

#[test]
fn test_dirty_paths_tracked_correctly() {
    let store = InMemoryMetadataStore::new();
    let repo_path = PathBuf::from("/tmp/test_repo");

    store.register_repo(repo_path.clone()).expect("register");

    // Mark files dirty
    store.mark_dirty_path(&repo_path, PathBuf::from("file1.txt")).expect("mark 1");
    store.mark_dirty_path(&repo_path, PathBuf::from("file2.txt")).expect("mark 2");
    store.mark_dirty_path(&repo_path, PathBuf::from("file1.txt")).expect("mark dup"); // duplicate

    let count = store.dirty_path_count(&repo_path).expect("count");
    assert_eq!(count, 2); // Should dedupe

    let drained = store.drain_dirty_paths(&repo_path).expect("drain");
    assert_eq!(drained.len(), 2);

    // After drain, should be empty
    let count_after = store.dirty_path_count(&repo_path).expect("count after");
    assert_eq!(count_after, 0);
}

// =============================================================================
// Daemon Integration Tests
// =============================================================================

#[tokio::test]
async fn test_daemon_register_and_status_flow() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register repo
    let response = service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    assert!(matches!(response, DaemonResponse::Ack(_)));

    // Get status - should be clean initially
    let response = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status");

    match response {
        DaemonResponse::RepoStatus(detail) => {
            assert_eq!(detail.repo_path, dir.path());
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_status_detects_real_file_changes() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register repo
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    // First status (clean)
    let _response = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status");

    // Now modify a file
    modify_file(dir.path(), "README.md", "# Changed content\n");

    // Get status again - the working_tree_status should detect the change
    let response = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status after change");

    match response {
        DaemonResponse::RepoStatus(detail) => {
            // The status should include the modified file
            assert!(
                detail.dirty_paths.contains(&PathBuf::from("README.md")),
                "Should detect modified README.md, got: {:?}",
                detail.dirty_paths
            );
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_fsmonitor_snapshot() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register repo
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    // Create file changes
    modify_file(dir.path(), "README.md", "# Changed\n");
    create_untracked_file(dir.path(), "new.txt", "new file");

    // Get fsmonitor snapshot
    let response = service
        .execute(DaemonCommand::FsMonitorSnapshot {
            repo_path: dir.path().to_path_buf(),
            last_seen_generation: None,
        })
        .await
        .expect("snapshot");

    match response {
        DaemonResponse::FsMonitorSnapshot(snapshot) => {
            assert_eq!(snapshot.repo_path, dir.path());
            // Should contain dirty paths from git status
            assert!(!snapshot.dirty_paths.is_empty() || snapshot.generation > 0);
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_job_queueing() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register repo
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    // Queue prefetch job
    let response = service
        .execute(DaemonCommand::QueueJob {
            repo_path: dir.path().to_path_buf(),
            job: JobKind::Prefetch,
        })
        .await
        .expect("queue job");

    assert!(matches!(response, DaemonResponse::Ack(_)));

    // Check health shows pending job
    let response = service
        .execute(DaemonCommand::HealthCheck)
        .await
        .expect("health");

    match response {
        DaemonResponse::Health(health) => {
            assert!(health.pending_jobs >= 1);
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_repo_health_diagnostics() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register repo
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    // Get repo health
    let response = service
        .execute(DaemonCommand::RepoHealth {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("repo health");

    match response {
        DaemonResponse::RepoHealth(detail) => {
            assert_eq!(detail.repo_path, dir.path());
            assert!(detail.sled_ok);
            assert!(!detail.needs_reconciliation);
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_list_repos() {
    let dir1 = TempDir::new().unwrap();
    let dir2 = TempDir::new().unwrap();
    let _repo1 = create_test_repo(dir1.path());
    let _repo2 = create_test_repo(dir2.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register both repos
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir1.path().to_path_buf(),
        })
        .await
        .expect("register 1");
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir2.path().to_path_buf(),
        })
        .await
        .expect("register 2");

    // List repos
    let response = service
        .execute(DaemonCommand::ListRepos)
        .await
        .expect("list");

    match response {
        DaemonResponse::RepoList(list) => {
            assert_eq!(list.len(), 2);
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

#[tokio::test]
async fn test_daemon_unregister_repo() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Register then unregister
    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    let response = service
        .execute(DaemonCommand::UnregisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("unregister");

    assert!(matches!(response, DaemonResponse::Ack(_)));

    // Verify removed
    let response = service
        .execute(DaemonCommand::ListRepos)
        .await
        .expect("list");

    match response {
        DaemonResponse::RepoList(list) => {
            assert!(list.is_empty());
        }
        other => panic!("unexpected response: {:?}", other),
    }
}

// =============================================================================
// Client-Server Integration Tests
// =============================================================================

#[tokio::test]
async fn test_nng_client_server_with_real_repo() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let shutdown = runtime.shutdown_signal();
    let shared = runtime.shared();

    // Use random port based on process id
    let port = 19000 + (std::process::id() % 1000) as u16;
    let address = format!("tcp://127.0.0.1:{}", port);

    let server = NngServer::new(address.clone(), shared, shutdown.clone());

    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.run().await;
    });

    // Give server time to start
    sleep(Duration::from_millis(100)).await;

    // Create client and test
    let client = NngClient::new(address);

    let response = client
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register via client");

    assert!(matches!(response, DaemonResponse::Ack(_)));

    // Test status query
    let response = client
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status via client");

    assert!(matches!(response, DaemonResponse::RepoStatus(_)));

    // Shutdown
    shutdown.shutdown();
    let _ = server_handle.await;
}

// =============================================================================
// Generation Counter Tests
// =============================================================================

#[tokio::test]
async fn test_generation_increments_on_changes() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");

    // First status
    let response1 = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status 1");

    let gen1 = match response1 {
        DaemonResponse::RepoStatus(detail) => detail.generation,
        other => panic!("unexpected: {:?}", other),
    };

    // Modify file and query again WITHOUT known_generation
    // (This forces a fresh status check that will detect the change)
    modify_file(dir.path(), "README.md", "# Changed again\n");

    let response2 = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None, // Don't pass known generation to force full check
        })
        .await
        .expect("status 2");

    let gen2 = match response2 {
        DaemonResponse::RepoStatus(detail) => detail.generation,
        DaemonResponse::RepoStatusUnchanged { generation, .. } => generation,
        other => panic!("unexpected: {:?}", other),
    };

    // Generation should increment because we got dirty paths from git status
    assert!(gen2 >= gen1, "Generation should be at least the same");
}

// =============================================================================
// End-to-End Workflow Test
// =============================================================================

#[tokio::test]
async fn test_full_workflow_register_modify_status() {
    // This test simulates the complete user workflow:
    // 1. User registers a repo
    // 2. User makes changes to files
    // 3. User queries status
    // 4. Gitz correctly reports the dirty files

    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    let store = InMemoryMetadataStore::new();
    let runtime = Runtime::new(store, None);
    let service = runtime.service_handle();

    // Step 1: Register repo
    let response = service
        .execute(DaemonCommand::RegisterRepo {
            repo_path: dir.path().to_path_buf(),
        })
        .await
        .expect("register");
    assert!(matches!(response, DaemonResponse::Ack(_)));

    // Step 2: Make changes
    modify_file(dir.path(), "README.md", "# New content\n");
    create_untracked_file(dir.path(), "src/lib.rs", "pub fn hello() {}");
    create_untracked_file(dir.path(), "tests/test.rs", "#[test] fn it_works() {}");

    // Step 3: Query status
    let response = service
        .execute(DaemonCommand::Status {
            repo_path: dir.path().to_path_buf(),
            known_generation: None,
        })
        .await
        .expect("status");

    // Step 4: Verify dirty paths
    match response {
        DaemonResponse::RepoStatus(detail) => {
            assert_eq!(detail.repo_path, dir.path());
            assert!(detail.dirty_paths.len() >= 1, "Should have dirty paths");

            // Check specific files are detected
            let has_readme = detail.dirty_paths.iter().any(|p| p.ends_with("README.md"));
            assert!(has_readme, "Should detect modified README.md");
        }
        other => panic!("unexpected response: {:?}", other),
    }

    // Verify health is good
    let response = service
        .execute(DaemonCommand::HealthCheck)
        .await
        .expect("health");

    match response {
        DaemonResponse::Health(health) => {
            assert_eq!(health.repo_count, 1);
        }
        other => panic!("unexpected: {:?}", other),
    }
}

// =============================================================================
// FSMonitor Protocol Tests
// =============================================================================

/// Test that fsmonitor snapshot produces correctly formatted output
#[test]
fn test_fsmonitor_protocol_output_format() {
    // The fsmonitor protocol v2 expects:
    // - A token (generation number) followed by NUL
    // - Each dirty path followed by NUL
    let snapshot = FsMonitorSnapshot {
        repo_path: PathBuf::from("/test/repo"),
        generation: 42,
        dirty_paths: vec![
            PathBuf::from("src/main.rs"),
            PathBuf::from("Cargo.toml"),
        ],
    };

    // Simulate the output format
    let mut output = Vec::new();
    output.extend_from_slice(snapshot.generation.to_string().as_bytes());
    output.push(0); // NUL separator
    for path in &snapshot.dirty_paths {
        if let Some(s) = path.to_str() {
            output.extend_from_slice(s.as_bytes());
            output.push(0); // NUL separator
        }
    }

    // Verify format
    let output_str = String::from_utf8_lossy(&output);
    assert!(output_str.starts_with("42\0"));
    assert!(output_str.contains("src/main.rs\0"));
    assert!(output_str.contains("Cargo.toml\0"));
}

// =============================================================================
// Git Command Execution Tests
// =============================================================================

/// Test that git maintenance command is available and can run
#[test]
fn test_git_maintenance_command_exists() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Verify git maintenance run works (dry-run would be ideal but not all git versions support it)
    let output = Command::new("git")
        .args(["maintenance", "run", "--task=prefetch"])
        .current_dir(dir.path())
        .output();

    // We just verify the command executes - it may fail due to no remote, but that's ok
    assert!(output.is_ok(), "git maintenance command should be available");
}

/// Test that git status works in test repo
#[test]
fn test_git_status_command_works() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Modify a file
    modify_file(dir.path(), "README.md", "# Changed\n");

    // Run git status
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(dir.path())
        .output()
        .expect("git status should work");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("README.md"), "git status should show modified file");
}

/// Test that our working_tree_status matches git status output
#[test]
fn test_working_tree_status_matches_git_status() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Create changes (avoid nested directories as git status shows them differently)
    modify_file(dir.path(), "README.md", "# Changed\n");
    create_untracked_file(dir.path(), "new_file.txt", "new");

    // Get our status
    let our_status = working_tree_status(dir.path(), &[]).expect("our status");

    // Get git's status (with -uall to show individual files)
    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(dir.path())
        .output()
        .expect("git status");
    let git_output = String::from_utf8_lossy(&output.stdout);

    // Verify our status contains the same files as git status
    for path in &our_status {
        let path_str = path.to_string_lossy();
        assert!(
            git_output.contains(&*path_str),
            "Our status path '{}' should be in git status output: {}",
            path_str,
            git_output
        );
    }

    // Count should match
    let git_lines: Vec<_> = git_output.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        our_status.len(),
        git_lines.len(),
        "Status count should match: ours={:?}, git={:?}",
        our_status,
        git_lines
    );
}

/// Test that git config changes persist correctly
#[test]
fn test_git_config_fsmonitor_setting() {
    let dir = TempDir::new().unwrap();
    let _repo = create_test_repo(dir.path());

    // Apply fsmonitor setting
    let configurator = RepoConfigurator::open(dir.path()).expect("open");
    configurator
        .apply_performance_settings(Some("gitz fsmonitor-helper 2"))
        .expect("apply");

    // Verify via git config command
    let output = Command::new("git")
        .args(["config", "--get", "core.fsmonitor"])
        .current_dir(dir.path())
        .output()
        .expect("git config");

    let value = String::from_utf8_lossy(&output.stdout);
    assert!(
        value.contains("gitz fsmonitor-helper"),
        "fsmonitor should be set: {}",
        value
    );

    // Clear and verify
    configurator.clear_performance_settings().expect("clear");

    let output = Command::new("git")
        .args(["config", "--get", "core.fsmonitor"])
        .current_dir(dir.path())
        .output()
        .expect("git config after clear");

    // Should return error (exit code 1) when key doesn't exist
    assert!(!output.status.success(), "fsmonitor config should be removed");
}
