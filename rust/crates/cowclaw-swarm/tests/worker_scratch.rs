use cowclaw_swarm::worker::scratch::ScratchWorktree;
use tempfile::TempDir;
use std::process::Command;

#[test]
fn scratch_creates_worktree_and_cleans_up() {
    let repo = TempDir::new().unwrap();
    let repo_path = repo.path();
    // init git repo with initial commit
    Command::new("git").args(["init", repo_path.to_str().unwrap()]).output().unwrap();
    Command::new("git").args(["-C", repo_path.to_str().unwrap(), "config", "user.email", "test@test.com"]).output().unwrap();
    Command::new("git").args(["-C", repo_path.to_str().unwrap(), "config", "user.name", "Test"]).output().unwrap();
    std::fs::write(repo_path.join("README.md"), "init").unwrap();
    Command::new("git").args(["-C", repo_path.to_str().unwrap(), "add", "."]).output().unwrap();
    Command::new("git").args(["-C", repo_path.to_str().unwrap(), "commit", "-m", "init"]).output().unwrap();

    let scratch_path = {
        let sw = ScratchWorktree::create(repo_path, "plan-01").unwrap();
        assert!(sw.path().exists());
        sw.path().to_path_buf()
    }; // drop here → cleanup
    assert!(!scratch_path.exists());
}
