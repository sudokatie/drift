//! Git repository data source
//!
//! Watches a git repository for commits, branch changes, and file activity.

use super::{DataPoint, Source};
use anyhow::{Context, Result};
use git2::{Repository, Status, StatusOptions};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Configuration for git source
#[derive(Debug, Clone)]
pub struct GitConfig {
    /// Path to git repository
    pub path: PathBuf,
    /// Poll interval for checking changes
    pub interval: Duration,
    /// Whether to watch for file changes (not just commits)
    pub watch_files: bool,
}

impl GitConfig {
    /// Create config from settings map
    pub fn from_settings(settings: &HashMap<String, serde_yaml::Value>) -> Result<Self> {
        let path = settings
            .get("path")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .context("git source requires 'path' setting")?;

        let interval_ms = settings
            .get("interval_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(5000); // 5 seconds default

        let watch_files = settings
            .get("watch")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(Self {
            path,
            interval: Duration::from_millis(interval_ms),
            watch_files,
        })
    }
}

/// Git repository state for change detection
#[derive(Debug, Clone, Default)]
struct GitState {
    /// Current HEAD commit hash
    head_commit: Option<String>,
    /// Current branch name
    branch: Option<String>,
    /// Number of modified files
    modified_count: usize,
    /// Number of staged files
    staged_count: usize,
    /// Total commits in history
    commit_count: usize,
}

/// Source that watches a git repository
pub struct GitSource {
    name: String,
    config: GitConfig,
    running: Arc<AtomicBool>,
    sender: broadcast::Sender<DataPoint>,
    task: Option<JoinHandle<()>>,
}

impl GitSource {
    /// Create a new git source
    pub fn new(name: impl Into<String>, config: GitConfig) -> Self {
        let (sender, _) = broadcast::channel(16);
        Self {
            name: name.into(),
            config,
            running: Arc::new(AtomicBool::new(false)),
            sender,
            task: None,
        }
    }

    /// Get current git state from repository
    fn get_git_state(repo: &Repository) -> Result<GitState> {
        let mut state = GitState::default();

        // Get HEAD commit
        if let Ok(head) = repo.head() {
            if let Some(oid) = head.target() {
                state.head_commit = Some(oid.to_string()[..8].to_string());
            }
            if head.is_branch() {
                state.branch = head.shorthand().map(|s| s.to_string());
            }
        }

        // Count commits (limit to avoid slow startup on large repos)
        if let Ok(mut revwalk) = repo.revwalk() {
            if revwalk.push_head().is_ok() {
                state.commit_count = revwalk.take(10000).count();
            }
        }

        // Get file status
        let mut opts = StatusOptions::new();
        opts.include_untracked(true);
        if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
            for entry in statuses.iter() {
                let status = entry.status();
                if status.intersects(
                    Status::INDEX_NEW
                        | Status::INDEX_MODIFIED
                        | Status::INDEX_DELETED
                        | Status::INDEX_RENAMED
                        | Status::INDEX_TYPECHANGE,
                ) {
                    state.staged_count += 1;
                }
                if status.intersects(
                    Status::WT_NEW
                        | Status::WT_MODIFIED
                        | Status::WT_DELETED
                        | Status::WT_RENAMED
                        | Status::WT_TYPECHANGE,
                ) {
                    state.modified_count += 1;
                }
            }
        }

        Ok(state)
    }

    /// Compare states and generate events
    fn detect_changes(old: &GitState, new: &GitState) -> Vec<String> {
        let mut events = Vec::new();

        // New commit detected
        if old.head_commit != new.head_commit && new.head_commit.is_some() {
            events.push("commit".to_string());
        }

        // Branch changed
        if old.branch != new.branch {
            events.push("branch_change".to_string());
        }

        // Files staged
        if new.staged_count > old.staged_count {
            events.push("staged".to_string());
        }

        // Files modified
        if new.modified_count > old.modified_count {
            events.push("file_change".to_string());
        }

        events
    }

    /// Convert state to DataPoint
    fn state_to_datapoint(name: &str, state: &GitState, events: Vec<String>) -> DataPoint {
        let mut point = DataPoint::new(name)
            .with_value("commit_count", state.commit_count as f64)
            .with_value("modified_count", state.modified_count as f64)
            .with_value("staged_count", state.staged_count as f64);

        // Activity score (0-100) based on uncommitted changes
        let activity = ((state.modified_count + state.staged_count) as f64 * 10.0).min(100.0);
        point = point.with_value("activity", activity);

        // Add events
        for event in events {
            point = point.with_event(&event);
        }

        point
    }
}

impl Source for GitSource {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Ok(());
        }

        // Verify repository exists
        Repository::open(&self.config.path)
            .with_context(|| format!("failed to open git repository: {:?}", self.config.path))?;

        self.running.store(true, Ordering::SeqCst);

        let name = self.name.clone();
        let path = self.config.path.clone();
        let interval = self.config.interval;
        let running = Arc::clone(&self.running);
        let sender = self.sender.clone();

        let task = tokio::spawn(async move {
            let mut previous_state = GitState::default();

            while running.load(Ordering::SeqCst) {
                match Repository::open(&path) {
                    Ok(repo) => match Self::get_git_state(&repo) {
                        Ok(state) => {
                            let events = Self::detect_changes(&previous_state, &state);
                            let point = Self::state_to_datapoint(&name, &state, events);
                            let _ = sender.send(point);
                            previous_state = state;
                        }
                        Err(e) => {
                            eprintln!("Git state error: {}", e);
                        }
                    },
                    Err(e) => {
                        eprintln!("Git repo error: {}", e);
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });

        self.task = Some(task);
        Ok(())
    }

    fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn subscribe(&self) -> broadcast::Receiver<DataPoint> {
        self.sender.subscribe()
    }
}

impl Drop for GitSource {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit
        {
            let sig = git2::Signature::now("Test", "test@test.com").unwrap();
            let tree_id = {
                let mut index = repo.index().unwrap();
                index.write_tree().unwrap()
            };
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn test_git_config_from_settings() {
        let mut settings = HashMap::new();
        settings.insert(
            "path".to_string(),
            serde_yaml::Value::String("/tmp/repo".to_string()),
        );
        settings.insert(
            "interval_ms".to_string(),
            serde_yaml::Value::Number(1000.into()),
        );
        settings.insert("watch".to_string(), serde_yaml::Value::Bool(true));

        let config = GitConfig::from_settings(&settings).unwrap();
        assert_eq!(config.path, PathBuf::from("/tmp/repo"));
        assert_eq!(config.interval, Duration::from_millis(1000));
        assert!(config.watch_files);
    }

    #[test]
    fn test_git_config_defaults() {
        let mut settings = HashMap::new();
        settings.insert(
            "path".to_string(),
            serde_yaml::Value::String("/tmp/repo".to_string()),
        );

        let config = GitConfig::from_settings(&settings).unwrap();
        assert_eq!(config.interval, Duration::from_millis(5000));
        assert!(config.watch_files);
    }

    #[test]
    fn test_git_config_missing_path() {
        let settings = HashMap::new();
        let result = GitConfig::from_settings(&settings);
        assert!(result.is_err());
    }

    #[test]
    fn test_git_state_basic() {
        let (_dir, repo) = create_test_repo();
        let state = GitSource::get_git_state(&repo).unwrap();

        assert!(state.head_commit.is_some());
        assert_eq!(state.commit_count, 1);
        assert_eq!(state.modified_count, 0);
        assert_eq!(state.staged_count, 0);
    }

    #[test]
    fn test_git_state_modified_files() {
        let (dir, repo) = create_test_repo();

        // Create an untracked file
        std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

        let state = GitSource::get_git_state(&repo).unwrap();
        // The untracked file should show up as modified (worktree new)
        assert!(state.modified_count > 0, "Expected modified files from untracked file");
    }

    #[test]
    fn test_detect_changes_commit() {
        let old = GitState {
            head_commit: Some("abc123".to_string()),
            ..Default::default()
        };
        let new = GitState {
            head_commit: Some("def456".to_string()),
            ..Default::default()
        };

        let events = GitSource::detect_changes(&old, &new);
        assert!(events.contains(&"commit".to_string()));
    }

    #[test]
    fn test_detect_changes_branch() {
        let old = GitState {
            branch: Some("main".to_string()),
            ..Default::default()
        };
        let new = GitState {
            branch: Some("feature".to_string()),
            ..Default::default()
        };

        let events = GitSource::detect_changes(&old, &new);
        assert!(events.contains(&"branch_change".to_string()));
    }

    #[test]
    fn test_detect_changes_files() {
        let old = GitState {
            modified_count: 0,
            ..Default::default()
        };
        let new = GitState {
            modified_count: 3,
            ..Default::default()
        };

        let events = GitSource::detect_changes(&old, &new);
        assert!(events.contains(&"file_change".to_string()));
    }

    #[test]
    fn test_state_to_datapoint() {
        let state = GitState {
            head_commit: Some("abc123".to_string()),
            branch: Some("main".to_string()),
            commit_count: 50,
            modified_count: 3,
            staged_count: 1,
        };

        let point = GitSource::state_to_datapoint("git", &state, vec!["commit".to_string()]);

        assert_eq!(point.source, "git");
        assert_eq!(point.values.get("commit_count"), Some(&50.0));
        assert_eq!(point.values.get("modified_count"), Some(&3.0));
        assert_eq!(point.values.get("staged_count"), Some(&1.0));
        assert_eq!(point.values.get("activity"), Some(&40.0)); // (3+1)*10
        assert!(point.events.contains(&"commit".to_string()));
    }

    #[test]
    fn test_git_source_creation() {
        let config = GitConfig {
            path: PathBuf::from("/tmp/test"),
            interval: Duration::from_secs(5),
            watch_files: true,
        };
        let source = GitSource::new("test_git", config);

        assert_eq!(source.name(), "test_git");
        assert!(!source.is_running());
    }
}
