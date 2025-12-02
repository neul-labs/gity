use std::{
    env, io,
    path::PathBuf,
    process::{Command, Stdio},
};

/// Prepared command used to spawn the daemon.
pub struct LaunchSpec {
    executable: PathBuf,
    args: Vec<String>,
    address: String,
}

impl LaunchSpec {
    pub fn new(address: impl Into<String>) -> io::Result<Self> {
        Ok(Self {
            executable: env::current_exe()?,
            args: vec!["daemon".into(), "run".into()],
            address: address.into(),
        })
    }

    pub fn spawn(&self) -> io::Result<()> {
        let mut command = self.build_command()?;
        command.spawn().map(|_| ())
    }

    fn build_command(&self) -> io::Result<Command> {
        let mut command = Command::new(&self.executable);
        for arg in &self.args {
            command.arg(arg);
        }
        command.env("GITZ_DAEMON_ADDR", &self.address);
        command.stdin(Stdio::null());
        command.stdout(Stdio::null());
        command.stderr(Stdio::null());

        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            unsafe {
                command.pre_exec(|| {
                    if libc::setsid() == -1 {
                        return Err(io::Error::last_os_error());
                    }
                    Ok(())
                });
            }
        }

        Ok(command)
    }

    #[cfg(test)]
    pub fn args(&self) -> &[String] {
        &self.args
    }

    #[cfg(test)]
    pub fn executable(&self) -> &PathBuf {
        &self.executable
    }
}

pub fn spawn_daemon(address: &str) -> io::Result<()> {
    LaunchSpec::new(address)?.spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_spec_uses_current_executable() {
        let spec = LaunchSpec::new("tcp://localhost:1234").expect("create spec");
        assert!(spec.executable().to_string_lossy().contains("gitz"));
        assert_eq!(spec.args(), &["daemon", "run"]);
    }
}
