use anyhow::{Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Demo tool to showcase gity's performance benefits
#[derive(Parser, Debug)]
#[command(name = "gity-demo")]
#[command(about = "Demonstrates gity's performance improvements for large git repositories")]
struct Args {
    /// Number of files to create in each test repository
    #[arg(long, default_value = "100000")]
    files: u64,

    /// Number of benchmark iterations per phase
    #[arg(long, default_value = "5")]
    iterations: u32,

    /// Skip repository creation (use existing repos)
    #[arg(long)]
    skip_setup: bool,

    /// Keep test repositories after demo (default: cleanup)
    #[arg(long)]
    keep_repos: bool,

    /// Stop gity daemon after demo completes (fully cleans up gity state)
    #[arg(long)]
    stop_daemon: bool,

    /// Base path for test repositories
    #[arg(long, default_value = "/tmp")]
    repo_path: PathBuf,

    /// Path to gity binary (defaults to searching PATH)
    #[arg(long)]
    gity_bin: Option<PathBuf>,
}

#[derive(Default)]
struct PhaseStats {
    with_gity_times: Vec<Duration>,
    without_gity_times: Vec<Duration>,
}

impl PhaseStats {
    fn avg_with_gity(&self) -> Duration {
        if self.with_gity_times.is_empty() {
            Duration::ZERO
        } else {
            let total: Duration = self.with_gity_times.iter().sum();
            total / self.with_gity_times.len() as u32
        }
    }

    fn avg_without_gity(&self) -> Duration {
        if self.without_gity_times.is_empty() {
            Duration::ZERO
        } else {
            let total: Duration = self.without_gity_times.iter().sum();
            total / self.without_gity_times.len() as u32
        }
    }

    fn speedup(&self) -> f64 {
        let avg_without = self.avg_without_gity().as_secs_f64();
        let avg_with = self.avg_with_gity().as_secs_f64();
        if avg_with > 0.0 {
            avg_without / avg_with
        } else {
            0.0
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    run_demo(args)
}

fn run_demo(args: Args) -> Result<()> {
    let gity_repo = args.repo_path.join("gity-demo-with");
    let baseline_repo = args.repo_path.join("gity-demo-without");

    // Find gity binary
    let gity_bin = args
        .gity_bin
        .clone()
        .unwrap_or_else(|| PathBuf::from("gity"));

    // Setup repositories if needed
    if !args.skip_setup {
        setup_repositories(&gity_repo, &baseline_repo, args.files)?;
    }

    // Verify repos exist
    if !gity_repo.exists() || !baseline_repo.exists() {
        anyhow::bail!(
            "Test repositories not found. Run without --skip-setup to create them.\n\
             Expected:\n  - {}\n  - {}",
            gity_repo.display(),
            baseline_repo.display()
        );
    }

    // Ensure gity daemon is running and register the repo
    println!("\nSetting up gity...");
    setup_gity(&gity_bin, &gity_repo)?;

    // Collect file paths for modification
    let file_paths = collect_file_paths(&gity_repo)?;
    let baseline_file_paths = collect_file_paths(&baseline_repo)?;

    println!("Found {} files in each repository\n", file_paths.len());

    // Give gity a moment to fully initialize file watchers
    println!("Waiting for gity to initialize watchers...");
    std::thread::sleep(Duration::from_secs(3));

    // Run the phased benchmark
    let (phase1_stats, phase2_stats, phase3_stats) = run_phased_benchmark(
        &gity_repo,
        &baseline_repo,
        &file_paths,
        &baseline_file_paths,
        args.iterations,
    )?;

    // Print summary
    print_summary(file_paths.len(), &phase1_stats, &phase2_stats, &phase3_stats);

    // Cleanup unless --keep-repos is specified
    if !args.keep_repos {
        println!("\nCleaning up...");

        // Unregister the gity-enabled repo
        if let Err(e) = unregister_gity(&gity_bin, &gity_repo) {
            eprintln!("Warning: Failed to unregister repo: {}", e);
        }

        // Verify unregistration worked
        if !verify_cleanup(&gity_bin, &gity_repo) {
            eprintln!("Warning: Repository may still be registered in gity");
        }

        // Delete repository directories
        if let Err(e) = std::fs::remove_dir_all(&gity_repo) {
            eprintln!("Warning: Failed to remove {}: {}", gity_repo.display(), e);
        }
        if let Err(e) = std::fs::remove_dir_all(&baseline_repo) {
            eprintln!("Warning: Failed to remove {}: {}", baseline_repo.display(), e);
        }

        // Stop daemon first (required to release database lock for compaction)
        if args.stop_daemon {
            if let Ok(status) = Command::new(&gity_bin).args(["daemon", "stop"]).status() {
                if status.success() {
                    println!("Gity daemon stopped.");
                    // Wait for daemon to fully release database lock
                    std::thread::sleep(Duration::from_millis(500));
                }
            }

            // Compact database to reclaim space (only possible after daemon stopped)
            println!("Compacting gity database...");
            if let Ok(output) = Command::new(&gity_bin).args(["db", "compact"]).output() {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Warning: Database compaction failed: {}", stderr.trim());
                }
            }
        }

        println!("Test repositories cleaned up.");
    } else {
        // Unregister but keep repos
        if let Err(e) = unregister_gity(&gity_bin, &gity_repo) {
            eprintln!("Warning: Failed to unregister repo: {}", e);
        }
        println!("\nWARNING: Keeping test repositories (--keep-repos was specified).");
        println!("  These can be large! Clean up manually when done:");
        println!("    rm -rf {} {}", gity_repo.display(), baseline_repo.display());
    }

    Ok(())
}

fn setup_repositories(gity_repo: &Path, baseline_repo: &Path, file_count: u64) -> Result<()> {
    println!("Setting up test repositories with {} files each...", file_count);
    println!("This may take several minutes for large file counts.\n");

    // Calculate directory structure
    let (top_dirs, sub_dirs, files_per_dir) = calculate_structure(file_count);

    println!(
        "Structure: {} top dirs x {} sub dirs x {} files = {} total files\n",
        top_dirs,
        sub_dirs,
        files_per_dir,
        top_dirs * sub_dirs * files_per_dir
    );

    // Create both repos sequentially to avoid I/O contention
    create_test_repo(gity_repo, top_dirs, sub_dirs, files_per_dir, "gity")?;
    create_test_repo(baseline_repo, top_dirs, sub_dirs, files_per_dir, "baseline")?;

    println!("\nRepositories created successfully!");
    Ok(())
}

fn calculate_structure(file_count: u64) -> (u64, u64, u64) {
    if file_count <= 1000 {
        (10, 10, (file_count / 100).max(1))
    } else if file_count <= 10000 {
        (100, 10, 10)
    } else if file_count <= 100000 {
        (100, 100, 10)
    } else {
        (1000, 100, 10)
    }
}

fn create_test_repo(
    repo_path: &Path,
    top_dirs: u64,
    sub_dirs: u64,
    files_per_dir: u64,
    label: &str,
) -> Result<()> {
    // Remove existing repo if any
    if repo_path.exists() {
        std::fs::remove_dir_all(repo_path)?;
    }
    std::fs::create_dir_all(repo_path)?;

    // Initialize git repo
    let repo = git2::Repository::init(repo_path)?;

    // Create progress bar
    let total = top_dirs * sub_dirs * files_per_dir;
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(&format!(
                "[{}] {{bar:40.cyan/blue}} {{pos}}/{{len}} files ({{eta}})",
                label
            ))?
            .progress_chars("##-"),
    );

    // Create files
    let mut file_count = 0u64;
    for top in 0..top_dirs {
        let top_dir = repo_path.join(format!("module{:04}", top));
        std::fs::create_dir_all(&top_dir)?;

        for sub in 0..sub_dirs {
            let sub_dir = top_dir.join(format!("sub{:03}", sub));
            std::fs::create_dir_all(&sub_dir)?;

            for file_idx in 0..files_per_dir {
                let file_path = sub_dir.join(format!("file{:02}.txt", file_idx));
                let content = format!(
                    "// File {} - module{:04}/sub{:03}/file{:02}.txt\nconst ID = {};\n",
                    file_count, top, sub, file_idx, file_count
                );
                std::fs::write(&file_path, content)?;
                file_count += 1;
                pb.inc(1);
            }
        }
    }

    pb.finish_with_message("Files created");

    // Stage all files and commit
    println!("[{}] Staging files...", label);
    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    println!("[{}] Creating initial commit...", label);
    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let sig = git2::Signature::now("Gity Demo", "demo@gity.dev")?;
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit with test files", &tree, &[])?;

    println!("[{}] Repository ready!", label);
    Ok(())
}

fn setup_gity(gity_bin: &Path, repo_path: &Path) -> Result<()> {
    // Start daemon if not running
    let status = Command::new(gity_bin)
        .args(["daemon", "start"])
        .status()
        .context("Failed to start gity daemon. Is gity installed?")?;

    if !status.success() {
        println!("Note: Daemon may already be running (which is fine)");
    }

    // Register the repo - this automatically configures fsmonitor and all performance settings
    // The daemon applies 7 git config settings via RepoConfigurator::apply_performance_settings()
    let output = Command::new(gity_bin)
        .args(["register", &repo_path.to_string_lossy()])
        .output()
        .context("Failed to register repo with gity")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.contains("already registered") {
            anyhow::bail!("Failed to register repo: {}", stderr);
        }
        println!("Note: Repository already registered (using existing registration)");
    }

    println!("Gity configured for: {}", repo_path.display());
    Ok(())
}

fn unregister_gity(gity_bin: &Path, repo_path: &Path) -> Result<()> {
    let output = Command::new(gity_bin)
        .args(["unregister", &repo_path.to_string_lossy()])
        .output()
        .context("Failed to run gity unregister")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Only warn, don't fail - repo might have been manually removed
        if !stderr.contains("not registered") {
            eprintln!("Warning: Failed to unregister repo: {}", stderr);
        }
    }
    Ok(())
}

fn verify_cleanup(gity_bin: &Path, repo_path: &Path) -> bool {
    let output = Command::new(gity_bin)
        .args(["list"])
        .output()
        .ok();

    match output {
        Some(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            !stdout.contains(&repo_path.to_string_lossy().to_string())
        }
        _ => true // Assume success if list command fails
    }
}

fn collect_file_paths(repo_path: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();

    fn visit_dir(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().map(|n| n != ".git").unwrap_or(true) {
                    visit_dir(&path, paths)?;
                }
            } else {
                paths.push(path);
            }
        }
        Ok(())
    }

    visit_dir(repo_path, &mut paths)?;
    Ok(paths)
}

fn run_phased_benchmark(
    gity_repo: &Path,
    baseline_repo: &Path,
    gity_files: &[PathBuf],
    baseline_files: &[PathBuf],
    iterations: u32,
) -> Result<(PhaseStats, PhaseStats, PhaseStats)> {
    println!("============================================================");
    println!("                  GITY PERFORMANCE DEMO");
    println!("============================================================\n");

    // Phase 1: Clean repository (no changes)
    println!("Phase 1: Clean Repository (no pending changes)");
    println!("  Scenario: Developer runs 'git status' on unchanged repo");
    println!("  This is the most common case - checking if anything changed.\n");

    let phase1_stats = run_phase(gity_repo, baseline_repo, iterations)?;
    print_phase_results("Phase 1", &phase1_stats);

    // Reset repos to clean state before Phase 2
    reset_repo_changes(gity_repo)?;
    reset_repo_changes(baseline_repo)?;
    std::thread::sleep(Duration::from_secs(1));

    // Phase 2: Single file change
    println!("\nPhase 2: Single File Change");
    println!("  Scenario: Developer edits one file, runs 'git status'");
    println!("  gity only needs to report 1 changed file.\n");

    // Modify exactly 1 file in both repos
    modify_files(&gity_files[0..1])?;
    modify_files(&baseline_files[0..1])?;
    std::thread::sleep(Duration::from_millis(500)); // Let gity detect the change

    let phase2_stats = run_phase(gity_repo, baseline_repo, iterations)?;
    print_phase_results("Phase 2", &phase2_stats);

    // Reset repos before Phase 3
    reset_repo_changes(gity_repo)?;
    reset_repo_changes(baseline_repo)?;
    std::thread::sleep(Duration::from_secs(1));

    // Phase 3: Small edit session (10 files)
    println!("\nPhase 3: Small Edit Session (10 files changed)");
    println!("  Scenario: Developer edits several files during a coding session");
    println!("  gity only needs to report 10 changed files.\n");

    // Modify 10 files in both repos
    let num_files = 10.min(gity_files.len());
    modify_files(&gity_files[0..num_files])?;
    modify_files(&baseline_files[0..num_files])?;
    std::thread::sleep(Duration::from_millis(500));

    let phase3_stats = run_phase(gity_repo, baseline_repo, iterations)?;
    print_phase_results("Phase 3", &phase3_stats);

    Ok((phase1_stats, phase2_stats, phase3_stats))
}

fn run_phase(gity_repo: &Path, baseline_repo: &Path, iterations: u32) -> Result<PhaseStats> {
    let mut stats = PhaseStats::default();

    println!("{:>10} {:>15} {:>15} {:>10}", "Iter", "With Gity", "Without Gity", "Speedup");
    println!("{}", "-".repeat(55));

    for i in 1..=iterations {
        let gity_time = time_git_status(gity_repo)?;
        let baseline_time = time_git_status(baseline_repo)?;

        stats.with_gity_times.push(gity_time);
        stats.without_gity_times.push(baseline_time);

        let speedup = baseline_time.as_secs_f64() / gity_time.as_secs_f64().max(0.001);

        println!(
            "{:>10} {:>12.2}ms {:>12.2}ms {:>9.1}x",
            i,
            gity_time.as_secs_f64() * 1000.0,
            baseline_time.as_secs_f64() * 1000.0,
            speedup
        );

        // Small delay between iterations
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(stats)
}

fn print_phase_results(phase_name: &str, stats: &PhaseStats) {
    println!("{}", "-".repeat(55));
    println!(
        "{:>10} {:>12.2}ms {:>12.2}ms {:>9.1}x",
        "Average:",
        stats.avg_with_gity().as_secs_f64() * 1000.0,
        stats.avg_without_gity().as_secs_f64() * 1000.0,
        stats.speedup()
    );
    println!(
        "\n  {} Result: {:.1}x speedup with gity",
        phase_name,
        stats.speedup()
    );
}

fn modify_files(files: &[PathBuf]) -> Result<()> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    for file in files {
        let content = format!("// Modified at {}\nexport const CHANGED = true;\n", timestamp);
        std::fs::write(file, content)?;
    }
    Ok(())
}

fn reset_repo_changes(repo_path: &Path) -> Result<()> {
    // Use git checkout to reset all changes
    Command::new("git")
        .args(["checkout", "."])
        .current_dir(repo_path)
        .output()?;
    Ok(())
}

fn time_git_status(repo_path: &Path) -> Result<Duration> {
    let start = Instant::now();
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_path)
        .output()?;

    let elapsed = start.elapsed();

    if !output.status.success() {
        anyhow::bail!("git status failed");
    }

    Ok(elapsed)
}

fn print_summary(
    total_files: usize,
    phase1: &PhaseStats,
    phase2: &PhaseStats,
    phase3: &PhaseStats,
) {
    println!("\n============================================================");
    println!("                        SUMMARY");
    println!("============================================================\n");

    println!("Total files in repository: {}\n", total_files);

    println!(
        "| {:<25} | {:>12} | {:>12} | {:>8} |",
        "Scenario", "With Gity", "Without Gity", "Speedup"
    );
    println!("|{:-<27}|{:-<14}|{:-<14}|{:-<10}|", "", "", "", "");

    println!(
        "| {:<25} | {:>9.1}ms | {:>9.1}ms | {:>7.1}x |",
        "Clean repo (0 changes)",
        phase1.avg_with_gity().as_secs_f64() * 1000.0,
        phase1.avg_without_gity().as_secs_f64() * 1000.0,
        phase1.speedup()
    );

    println!(
        "| {:<25} | {:>9.1}ms | {:>9.1}ms | {:>7.1}x |",
        "Single file changed",
        phase2.avg_with_gity().as_secs_f64() * 1000.0,
        phase2.avg_without_gity().as_secs_f64() * 1000.0,
        phase2.speedup()
    );

    println!(
        "| {:<25} | {:>9.1}ms | {:>9.1}ms | {:>7.1}x |",
        "10 files changed",
        phase3.avg_with_gity().as_secs_f64() * 1000.0,
        phase3.avg_without_gity().as_secs_f64() * 1000.0,
        phase3.speedup()
    );

    println!();

    let avg_speedup = (phase1.speedup() + phase2.speedup() + phase3.speedup()) / 3.0;

    if avg_speedup > 5.0 {
        println!("Conclusion: gity provides {:.0}x average speedup by skipping unchanged files!", avg_speedup);
    } else if avg_speedup > 1.5 {
        println!("Conclusion: gity provides {:.1}x average speedup.", avg_speedup);
        println!("\nNote: For more dramatic results, try with more files (--files 500000)");
    } else {
        println!("Note: Speedup is modest with this file count.");
        println!("      gity shines with larger repositories (500k+ files).");
        println!("      Try: --files 500000 for more dramatic results.");
    }

    println!("\n============================================================");
}
