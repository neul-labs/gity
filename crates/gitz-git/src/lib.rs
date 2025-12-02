use git2::{Config, Repository, Status, StatusOptions};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};
use thiserror::Error;

const PARTIAL_CLONE_FILTER: &str = "blob:none";

#[derive(Debug, Error)]
pub enum GitError {
    #[error("failed to open git repository at {path:?}: {source}")]
    OpenRepo {
        path: PathBuf,
        #[source]
        source: git2::Error,
    },
    #[error("git config error: {0}")]
    Config(String),
    #[error("git status error: {source}")]
    Status {
        #[from]
        source: git2::Error,
    },
}

pub struct RepoConfigurator {
    repo: Repository,
}

impl RepoConfigurator {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo_path = path.as_ref().to_path_buf();
        let repo = Repository::open(&repo_path).map_err(|source| GitError::OpenRepo {
            path: repo_path.clone(),
            source,
        })?;
        Ok(Self { repo })
    }

    pub fn apply_performance_settings(
        &self,
        fsmonitor_helper: Option<&str>,
    ) -> Result<(), GitError> {
        let mut config = self.repository_config()?;
        if let Some(helper) = fsmonitor_helper {
            config
                .set_str("core.fsmonitor", helper)
                .map_err(map_config_err)?;
        } else {
            remove_entry(&mut config, "core.fsmonitor")?;
        }
        config
            .set_bool("core.untrackedCache", true)
            .map_err(map_config_err)?;
        config
            .set_bool("feature.manyFiles", true)
            .map_err(map_config_err)?;
        config
            .set_bool("fetch.writeCommitGraph", true)
            .map_err(map_config_err)?;
        config
            .set_bool("gc.writeCommitGraph", true)
            .map_err(map_config_err)?;
        config
            .set_bool("remote.origin.promisor", true)
            .map_err(map_config_err)?;
        config
            .set_str("remote.origin.partialclonefilter", PARTIAL_CLONE_FILTER)
            .map_err(map_config_err)?;
        Ok(())
    }

    pub fn clear_performance_settings(&self) -> Result<(), GitError> {
        let mut config = self.repository_config()?;
        remove_entry(&mut config, "core.fsmonitor")?;
        remove_entry(&mut config, "core.untrackedCache")?;
        remove_entry(&mut config, "feature.manyFiles")?;
        remove_entry(&mut config, "fetch.writeCommitGraph")?;
        remove_entry(&mut config, "gc.writeCommitGraph")?;
        remove_entry(&mut config, "remote.origin.promisor")?;
        remove_entry(&mut config, "remote.origin.partialclonefilter")?;
        Ok(())
    }

    fn repository_config(&self) -> Result<Config, GitError> {
        self.repo
            .config()
            .map_err(|err| GitError::Config(err.to_string()))
    }
}

fn remove_entry(config: &mut Config, key: &str) -> Result<(), GitError> {
    match config.remove(key) {
        Ok(()) => Ok(()),
        Err(err) if err.code() == git2::ErrorCode::NotFound => Ok(()),
        Err(err) => Err(GitError::Config(err.to_string())),
    }
}

fn map_config_err(err: git2::Error) -> GitError {
    GitError::Config(err.to_string())
}

/// Returns the set of paths that Git currently considers dirty.
pub fn working_tree_status(
    repo_path: &Path,
    focus_paths: &[PathBuf],
) -> Result<Vec<PathBuf>, GitError> {
    let repo = Repository::open(repo_path).map_err(|source| GitError::OpenRepo {
        path: repo_path.to_path_buf(),
        source,
    })?;

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .renames_head_to_index(true)
        .exclude_submodules(true);

    if !focus_paths.is_empty() {
        for path in focus_paths {
            if let Some(spec) = path.to_str() {
                options.pathspec(spec);
            }
        }
    }

    let statuses = repo.statuses(Some(&mut options))?;
    let mut dirty = BTreeSet::new();
    for entry in statuses.iter() {
        let status = entry.status();
        if status == Status::CURRENT || status.contains(Status::IGNORED) {
            continue;
        }
        if let Some(path) = entry.path() {
            dirty.insert(PathBuf::from(path));
        }
    }

    Ok(dirty.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;
    use std::fs;

    #[test]
    fn applies_and_clears_settings() {
        let dir = tempfile::tempdir().unwrap();
        Repository::init(dir.path()).expect("init repo");
        let configurator = RepoConfigurator::open(dir.path()).expect("open repo");
        let helper_cmd = "gitz fsmonitor-helper";
        configurator
            .apply_performance_settings(Some(helper_cmd))
            .expect("apply settings");
        let config = read_config(dir.path());
        assert!(config.contains(helper_cmd));
        assert!(config.contains(PARTIAL_CLONE_FILTER));

        configurator
            .clear_performance_settings()
            .expect("clear settings");
        let config_after = read_config(dir.path());
        assert!(!config_after.contains(helper_cmd));
    }

    #[test]
    fn working_tree_status_detects_dirty_paths() {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).expect("init repo");
        let tracked = dir.path().join("tracked.txt");
        fs::write(&tracked, "hello").unwrap();

        let mut index = repo.index().expect("index");
        index.add_path(Path::new("tracked.txt")).expect("add path");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("tree");
        let sig = Signature::now("Gitz", "gitz@example.com").expect("signature");
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .expect("commit");

        fs::write(&tracked, "changed").unwrap();
        let status = working_tree_status(dir.path(), &[]).expect("status");
        assert_eq!(status, vec![PathBuf::from("tracked.txt")]);

        let filtered =
            working_tree_status(dir.path(), &[PathBuf::from("tracked.txt")]).expect("filtered");
        assert_eq!(filtered, vec![PathBuf::from("tracked.txt")]);
    }

    fn read_config(path: &Path) -> String {
        fs::read_to_string(path.join(".git/config")).expect("read config")
    }
}
