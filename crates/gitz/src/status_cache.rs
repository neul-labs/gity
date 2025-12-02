use gitz_ipc::RepoStatusDetail;
use sha1::{Digest, Sha1};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct StatusCache {
    dir: PathBuf,
}

impl StatusCache {
    pub fn new(base: &Path) -> io::Result<Self> {
        let dir = base.join("status_cache");
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn load(&self, repo_path: &Path) -> io::Result<Option<RepoStatusDetail>> {
        let path = self.cache_path(repo_path);
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)?;
        let detail = serde_json::from_str(&data)?;
        Ok(Some(detail))
    }

    pub fn store(&self, detail: &RepoStatusDetail) -> io::Result<()> {
        let path = self.cache_path(&detail.repo_path);
        let data = serde_json::to_string(detail)?;
        fs::write(path, data)?;
        Ok(())
    }

    fn cache_path(&self, repo_path: &Path) -> PathBuf {
        let mut hasher = Sha1::new();
        hasher.update(repo_path.to_string_lossy().as_bytes());
        let filename = format!("{}.json", hex::encode(hasher.finalize()));
        self.dir.join(filename)
    }
}
