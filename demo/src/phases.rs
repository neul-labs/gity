//! Workflow phases for the gity demo.
//!
//! Each phase represents a part of a developer's workflow.

use crate::tui::{DemoState, PhaseResult, Term};
use anyhow::Result;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// Run all demo phases in sequence.
pub fn run_all_phases(
    terminal: &mut Term,
    state: &mut DemoState,
    gity_repo: &Path,
    baseline_repo: &Path,
    gity_bin: &Path,
) -> Result<()> {
    // ACT 1: Morning Check-in
    run_phase_1(terminal, state, gity_repo, baseline_repo)?;

    // ACT 2: Coding Session
    run_phase_2(terminal, state, gity_repo, baseline_repo)?;

    // ACT 3: Commit Flow
    run_phase_3(terminal, state, gity_repo, baseline_repo)?;

    // ACT 4: Branch Switching
    run_phase_4(terminal, state, gity_repo, baseline_repo)?;

    // ACT 5: Background Magic
    run_phase_5(terminal, state, gity_bin)?;

    Ok(())
}

/// ACT 1: The Morning Check-in
/// Show git status on clean repo with race visualization.
pub fn run_phase_1(
    terminal: &mut Term,
    state: &mut DemoState,
    gity_repo: &Path,
    baseline_repo: &Path,
) -> Result<()> {
    state.phase = 1;
    state.phase_name = "The Morning Check-in".to_string();
    state.phase_description = "Developer runs 'git status' on unchanged repo".to_string();
    state.show_race = true;
    state.race_progress_gity = 0.0;
    state.race_progress_baseline = 0.0;
    state.status_message = "Running git status race...".to_string();

    // Render initial state
    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_millis(500));

    // Run gity version (fast)
    let gity_start = Instant::now();
    let _ = run_git_status(gity_repo);
    let gity_elapsed = gity_start.elapsed();
    state.gity_time_ms = gity_elapsed.as_secs_f64() * 1000.0;
    state.race_progress_gity = 1.0;

    // Animate gity completion
    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Run baseline (slow) with progress simulation
    let baseline_start = Instant::now();

    // Start baseline in background thread
    let baseline_repo_clone = baseline_repo.to_path_buf();
    let handle = std::thread::spawn(move || {
        run_git_status(&baseline_repo_clone)
    });

    // Animate progress while waiting
    let estimated_baseline_ms = state.gity_time_ms * 15.0; // Estimate 15x slower
    while !handle.is_finished() {
        let elapsed = baseline_start.elapsed().as_secs_f64() * 1000.0;
        state.race_progress_baseline = (elapsed / estimated_baseline_ms).min(0.95);
        terminal.draw(|f| crate::tui::render_demo(f, state))?;
        std::thread::sleep(Duration::from_millis(50));

        if crate::tui::check_for_quit()? {
            return Ok(());
        }
    }

    let baseline_elapsed = baseline_start.elapsed();
    state.baseline_time_ms = baseline_elapsed.as_secs_f64() * 1000.0;
    state.race_progress_baseline = 1.0;

    // Record result
    state.results.push(PhaseResult {
        name: "git status (clean)".to_string(),
        gity_ms: state.gity_time_ms,
        baseline_ms: state.baseline_time_ms,
    });

    // Show final state
    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(2));

    state.show_race = false;
    Ok(())
}

/// ACT 2: The Coding Session
/// Simulate editing files and checking status.
pub fn run_phase_2(
    terminal: &mut Term,
    state: &mut DemoState,
    gity_repo: &Path,
    baseline_repo: &Path,
) -> Result<()> {
    state.phase = 2;
    state.phase_name = "The Coding Session".to_string();
    state.phase_description = "Developer edits 5 files, runs 'git status'".to_string();
    state.status_message = "Simulating file edits...".to_string();
    state.gity_time_ms = 0.0;
    state.baseline_time_ms = 0.0;

    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_millis(500));

    // Modify 5 files in both repos
    modify_files(gity_repo, 5)?;
    modify_files(baseline_repo, 5)?;

    // Let gity detect changes
    std::thread::sleep(Duration::from_millis(200));

    state.status_message = "Running git status after edits...".to_string();
    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Time both
    let (gity_ms, baseline_ms) = time_both_repos(gity_repo, baseline_repo)?;
    state.gity_time_ms = gity_ms;
    state.baseline_time_ms = baseline_ms;

    state.results.push(PhaseResult {
        name: "git status (5 files)".to_string(),
        gity_ms,
        baseline_ms,
    });

    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(2));

    // Reset changes
    reset_repo(gity_repo)?;
    reset_repo(baseline_repo)?;

    Ok(())
}

/// ACT 3: The Commit Flow
/// Time git add and commit operations.
pub fn run_phase_3(
    terminal: &mut Term,
    state: &mut DemoState,
    gity_repo: &Path,
    baseline_repo: &Path,
) -> Result<()> {
    state.phase = 3;
    state.phase_name = "The Commit Flow".to_string();
    state.phase_description = "Developer stages and commits changes".to_string();
    state.status_message = "Preparing commit...".to_string();
    state.gity_time_ms = 0.0;
    state.baseline_time_ms = 0.0;

    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Modify files
    modify_files(gity_repo, 3)?;
    modify_files(baseline_repo, 3)?;
    std::thread::sleep(Duration::from_millis(200));

    state.status_message = "Running git add...".to_string();
    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Time git add
    let gity_add = time_git_command(gity_repo, &["add", "."])?;
    let baseline_add = time_git_command(baseline_repo, &["add", "."])?;

    state.gity_time_ms = gity_add;
    state.baseline_time_ms = baseline_add;

    state.results.push(PhaseResult {
        name: "git add".to_string(),
        gity_ms: gity_add,
        baseline_ms: baseline_add,
    });

    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(1));

    // Reset for clean state
    run_git_command(gity_repo, &["reset", "HEAD"])?;
    run_git_command(baseline_repo, &["reset", "HEAD"])?;
    reset_repo(gity_repo)?;
    reset_repo(baseline_repo)?;

    Ok(())
}

/// ACT 4: Branch Switching
/// Time branch operations.
pub fn run_phase_4(
    terminal: &mut Term,
    state: &mut DemoState,
    gity_repo: &Path,
    baseline_repo: &Path,
) -> Result<()> {
    state.phase = 4;
    state.phase_name = "Branch Switching".to_string();
    state.phase_description = "Developer switches to a feature branch".to_string();
    state.status_message = "Creating feature branch...".to_string();
    state.gity_time_ms = 0.0;
    state.baseline_time_ms = 0.0;

    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Create and switch to feature branch
    run_git_command(gity_repo, &["checkout", "-b", "feature-demo"])?;
    run_git_command(baseline_repo, &["checkout", "-b", "feature-demo"])?;

    state.status_message = "Switching back to main...".to_string();
    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Time checkout back to main
    let gity_checkout = time_git_command(gity_repo, &["checkout", "master"])?;
    let baseline_checkout = time_git_command(baseline_repo, &["checkout", "master"])?;

    state.gity_time_ms = gity_checkout;
    state.baseline_time_ms = baseline_checkout;

    state.results.push(PhaseResult {
        name: "git checkout".to_string(),
        gity_ms: gity_checkout,
        baseline_ms: baseline_checkout,
    });

    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(1));

    // Cleanup branch
    run_git_command(gity_repo, &["branch", "-D", "feature-demo"]).ok();
    run_git_command(baseline_repo, &["branch", "-D", "feature-demo"]).ok();

    Ok(())
}

/// ACT 5: The Background Magic
/// Show daemon features.
pub fn run_phase_5(
    terminal: &mut Term,
    state: &mut DemoState,
    _gity_bin: &Path,
) -> Result<()> {
    state.phase = 5;
    state.phase_name = "The Background Magic".to_string();
    state.phase_description = "What gity does automatically in the background".to_string();
    state.status_message = "Showing background features...".to_string();
    state.gity_time_ms = 0.0;
    state.baseline_time_ms = 0.0;

    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(1));

    // Show daemon status
    state.status_message = "File watching: Active | Prefetch: Scheduled | Maintenance: Idle".to_string();
    terminal.draw(|f| crate::tui::render_demo(f, state))?;
    std::thread::sleep(Duration::from_secs(2));

    Ok(())
}

/// Show summary screen.
pub fn show_summary(
    terminal: &mut Term,
    state: &mut DemoState,
) -> Result<()> {
    state.phase = 0;
    state.phase_name = String::new();
    state.phase_description = "Demo Complete".to_string();
    state.status_message = "Press any key to exit".to_string();

    terminal.draw(|f| crate::tui::render_demo(f, state))?;

    // Wait for key press
    crate::tui::wait_for_key()?;

    Ok(())
}

// Helper functions

fn run_git_status(repo: &Path) -> Result<()> {
    Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo)
        .output()?;
    Ok(())
}

fn run_git_command(repo: &Path, args: &[&str]) -> Result<()> {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()?;
    Ok(())
}

fn time_git_command(repo: &Path, args: &[&str]) -> Result<f64> {
    let start = Instant::now();
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()?;
    Ok(start.elapsed().as_secs_f64() * 1000.0)
}

fn time_both_repos(gity_repo: &Path, baseline_repo: &Path) -> Result<(f64, f64)> {
    let gity_ms = time_git_command(gity_repo, &["status", "--porcelain"])?;
    let baseline_ms = time_git_command(baseline_repo, &["status", "--porcelain"])?;
    Ok((gity_ms, baseline_ms))
}

fn modify_files(repo: &Path, count: usize) -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();

    // Find some files to modify
    let mut modified = 0;
    for entry in walkdir(repo, 3)? {
        if modified >= count {
            break;
        }
        if entry.is_file() && !entry.to_string_lossy().contains(".git") {
            let content = format!("// Modified at {}\nexport const CHANGED = true;\n", timestamp);
            std::fs::write(&entry, content)?;
            modified += 1;
        }
    }
    Ok(())
}

fn walkdir(dir: &Path, depth: usize) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    if depth == 0 {
        return Ok(files);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !path.file_name().map(|n| n == ".git").unwrap_or(false) {
            files.extend(walkdir(&path, depth - 1)?);
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

fn reset_repo(repo: &Path) -> Result<()> {
    Command::new("git")
        .args(["checkout", "."])
        .current_dir(repo)
        .output()?;
    Ok(())
}
