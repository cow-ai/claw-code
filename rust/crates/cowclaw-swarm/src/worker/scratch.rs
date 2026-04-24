use std::path::{Path, PathBuf};
use git2::Repository;

pub struct ScratchWorktree {
    repo_root: PathBuf,
    path: PathBuf,
    name: String,
}

impl ScratchWorktree {
    pub fn create(repo_root: &Path, plan_id: &str) -> crate::Result<Self> {
        let name = format!("scratch-{}", plan_id.replace('/', "-"));
        let scratch_dir = repo_root.join(".cowclaw").join("scratch").join(&name);
        std::fs::create_dir_all(scratch_dir.parent().unwrap())?;
        let repo = Repository::open(repo_root)?;
        repo.worktree(&name, &scratch_dir, None)?;
        Ok(Self { repo_root: repo_root.to_path_buf(), path: scratch_dir, name })
    }

    pub fn path(&self) -> &Path { &self.path }

    pub fn keep(self) -> PathBuf {
        let path = self.path.clone();
        std::mem::forget(self);
        path
    }
}

impl Drop for ScratchWorktree {
    fn drop(&mut self) {
        let _ = std::process::Command::new("git")
            .current_dir(&self.repo_root)
            .args(["worktree", "remove", "--force", &self.name])
            .status();
    }
}
