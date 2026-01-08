use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

/// Centralized helper for resolving config/data/log directories.
#[allow(dead_code)]
pub struct GityPaths {
    base: PathBuf,
    config: PathBuf,
    data: PathBuf,
    logs: PathBuf,
}

#[allow(dead_code)]
impl GityPaths {
    pub fn discover() -> io::Result<Self> {
        let base = resolve_base_dir()?;
        let config = base.join("config");
        let data = base.join("data");
        let logs = base.join("logs");

        for dir in [&base, &config, &data, &logs] {
            fs::create_dir_all(dir)?;
        }

        Ok(Self {
            base,
            config,
            data,
            logs,
        })
    }

    pub fn base_dir(&self) -> &Path {
        &self.base
    }

    pub fn config_dir(&self) -> &Path {
        &self.config
    }

    pub fn data_dir(&self) -> &Path {
        &self.data
    }

    pub fn logs_dir(&self) -> &Path {
        &self.logs
    }

    pub fn daemon_log_path(&self) -> PathBuf {
        self.logs.join("daemon.log")
    }
}

fn resolve_base_dir() -> io::Result<PathBuf> {
    if let Ok(custom) = env::var("GITY_HOME") {
        return Ok(PathBuf::from(custom));
    }
    #[cfg(unix)]
    {
        let home = dirs::home_dir().unwrap_or_else(env::temp_dir);
        Ok(home.join(".gity"))
    }
    #[cfg(not(unix))]
    {
        let base = dirs::data_dir().unwrap_or_else(env::temp_dir);
        Ok(base.join("Gity"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honors_gity_home_env() {
        let dir = tempfile::tempdir().unwrap();
        unsafe {
            env::set_var("GITY_HOME", dir.path());
        }
        let paths = GityPaths::discover().expect("discover paths");
        assert!(paths.base_dir().starts_with(dir.path()));
        assert!(paths.config_dir().exists());
        assert!(paths.logs_dir().exists());
        assert!(paths.data_dir().exists());
        unsafe {
            env::remove_var("GITY_HOME");
        }
    }
}
