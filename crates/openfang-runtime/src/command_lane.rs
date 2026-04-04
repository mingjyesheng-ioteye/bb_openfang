//! Command lane system — lane-based command queue with concurrency control.
//!
//! Routes different types of work through separate lanes with independent
//! concurrency limits to prevent starvation:
//! - Main: user messages (serialized, 1 at a time)
//! - Cron: scheduled jobs (2 concurrent)
//! - Subagent: spawned child agents (3 concurrent)

use std::sync::Arc;
use tokio::sync::Semaphore;

/// High-level shell command safety class used by coding loops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommandClass {
    ReadOnly,
    Mutating,
}

/// Classify shell command intent for lane and warning decisions.
pub fn classify_shell_command(command: &str) -> ShellCommandClass {
    let tokens = match shlex::split(command) {
        Some(t) if !t.is_empty() => t,
        _ => return ShellCommandClass::Mutating,
    };

    let bin = tokens[0].to_lowercase();
    let ro_bins = [
        "ls", "cat", "head", "tail", "pwd", "echo", "grep", "rg", "find", "tree", "wc",
        "stat", "du", "df", "ps", "top",
    ];
    if ro_bins.contains(&bin.as_str()) {
        return ShellCommandClass::ReadOnly;
    }

    if bin == "git" {
        let sub = tokens.get(1).map(|s| s.to_lowercase());
        let ro_git = [
            "status", "diff", "show", "log", "branch", "rev-parse", "grep", "ls-files",
            "remote", "tag", "blame",
        ];
        if sub.as_deref().is_some_and(|s| ro_git.contains(&s)) {
            return ShellCommandClass::ReadOnly;
        }
    }

    ShellCommandClass::Mutating
}

/// Produce non-blocking warning guidance for risky or stall-prone shell commands.
pub fn shell_command_warning(command: &str, class: ShellCommandClass) -> Option<String> {
    let cmd = command.to_lowercase();

    let destructive_patterns = [
        "rm -rf",
        "mkfs",
        "shutdown",
        "reboot",
        "poweroff",
        "terraform destroy",
        "kubectl delete",
        "drop database",
        "git reset --hard",
        "git clean -fd",
        "git push --force",
    ];
    if destructive_patterns.iter().any(|p| cmd.contains(p)) {
        return Some(
            "This looks destructive (filesystem, infra, database, or git history rewrite). Verify target scope and confirm intent before continuing."
                .to_string(),
        );
    }

    if let Some(sleep_secs) = parse_sleep_seconds(&cmd) {
        if sleep_secs >= 30 {
            return Some(
                "Long blocking sleep detected. Prefer non-blocking workflows (e.g., process_start + process_poll) to avoid stalling coding loops."
                    .to_string(),
            );
        }
    }

    if cmd.contains(" tail -f") || cmd.starts_with("tail -f") || cmd.contains(" watch ") {
        return Some(
            "Streaming/monitoring command detected. Consider running it in a background process and polling output instead of blocking the turn."
                .to_string(),
        );
    }

    if class == ShellCommandClass::Mutating && cmd.contains("git ") && cmd.contains(" checkout ") {
        return Some(
            "Git checkout changes working tree state. Re-read affected files before subsequent edits to avoid stale writes."
                .to_string(),
        );
    }

    None
}

fn parse_sleep_seconds(command_lc: &str) -> Option<u64> {
    let tokens: Vec<&str> = command_lc.split_whitespace().collect();
    for idx in 0..tokens.len() {
        if tokens[idx] == "sleep" {
            let next = tokens.get(idx + 1)?;
            if let Ok(secs) = next.parse::<u64>() {
                return Some(secs);
            }
        }
    }
    None
}

/// Command lane type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lane {
    /// User-facing message processing (1 concurrent).
    Main,
    /// Cron/scheduled job execution (2 concurrent).
    Cron,
    /// Subagent spawn/call execution (3 concurrent).
    Subagent,
}

impl std::fmt::Display for Lane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Lane::Main => write!(f, "main"),
            Lane::Cron => write!(f, "cron"),
            Lane::Subagent => write!(f, "subagent"),
        }
    }
}

/// Lane occupancy snapshot.
#[derive(Debug, Clone)]
pub struct LaneOccupancy {
    /// Lane type.
    pub lane: Lane,
    /// Current number of active tasks.
    pub active: u32,
    /// Maximum concurrent tasks.
    pub capacity: u32,
}

/// Command queue with lane-based concurrency control.
#[derive(Debug, Clone)]
pub struct CommandQueue {
    main_sem: Arc<Semaphore>,
    cron_sem: Arc<Semaphore>,
    subagent_sem: Arc<Semaphore>,
    main_capacity: u32,
    cron_capacity: u32,
    subagent_capacity: u32,
}

impl CommandQueue {
    /// Create a new command queue with default capacities.
    pub fn new() -> Self {
        Self {
            main_sem: Arc::new(Semaphore::new(1)),
            cron_sem: Arc::new(Semaphore::new(2)),
            subagent_sem: Arc::new(Semaphore::new(3)),
            main_capacity: 1,
            cron_capacity: 2,
            subagent_capacity: 3,
        }
    }

    /// Create with custom capacities.
    pub fn with_capacities(main: u32, cron: u32, subagent: u32) -> Self {
        Self {
            main_sem: Arc::new(Semaphore::new(main as usize)),
            cron_sem: Arc::new(Semaphore::new(cron as usize)),
            subagent_sem: Arc::new(Semaphore::new(subagent as usize)),
            main_capacity: main,
            cron_capacity: cron,
            subagent_capacity: subagent,
        }
    }

    /// Submit work to a lane. Acquires a permit, executes the future, releases.
    ///
    /// Returns `Err` if the semaphore is closed (shutdown).
    pub async fn submit<F, T>(&self, lane: Lane, work: F) -> Result<T, String>
    where
        F: std::future::Future<Output = T>,
    {
        let sem = self.semaphore_for(lane);
        let _permit = sem
            .acquire()
            .await
            .map_err(|_| format!("Lane {} is closed", lane))?;

        Ok(work.await)
    }

    /// Try to submit work without waiting (non-blocking).
    ///
    /// Returns `None` if the lane is at capacity.
    pub async fn try_submit<F, T>(&self, lane: Lane, work: F) -> Option<T>
    where
        F: std::future::Future<Output = T>,
    {
        let sem = self.semaphore_for(lane);
        let _permit = sem.try_acquire().ok()?;
        Some(work.await)
    }

    /// Get current occupancy for all lanes.
    pub fn occupancy(&self) -> Vec<LaneOccupancy> {
        vec![
            LaneOccupancy {
                lane: Lane::Main,
                active: self.main_capacity - self.main_sem.available_permits() as u32,
                capacity: self.main_capacity,
            },
            LaneOccupancy {
                lane: Lane::Cron,
                active: self.cron_capacity - self.cron_sem.available_permits() as u32,
                capacity: self.cron_capacity,
            },
            LaneOccupancy {
                lane: Lane::Subagent,
                active: self.subagent_capacity - self.subagent_sem.available_permits() as u32,
                capacity: self.subagent_capacity,
            },
        ]
    }

    fn semaphore_for(&self, lane: Lane) -> &Arc<Semaphore> {
        match lane {
            Lane::Main => &self.main_sem,
            Lane::Cron => &self.cron_sem,
            Lane::Subagent => &self.subagent_sem,
        }
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_main_lane_serialization() {
        let queue = CommandQueue::new();
        let counter = Arc::new(AtomicU32::new(0));

        // Main lane has capacity 1 — tasks should serialize
        let c1 = counter.clone();
        let result = queue
            .submit(Lane::Main, async move {
                c1.fetch_add(1, Ordering::SeqCst);
                42
            })
            .await;

        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_cron_lane_parallel() {
        let queue = Arc::new(CommandQueue::new());
        let counter = Arc::new(AtomicU32::new(0));

        let mut handles = Vec::new();
        for _ in 0..2 {
            let q = queue.clone();
            let c = counter.clone();
            handles.push(tokio::spawn(async move {
                q.submit(Lane::Cron, async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                })
                .await
            }));
        }

        for h in handles {
            h.await.unwrap().unwrap();
        }
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_occupancy() {
        let queue = CommandQueue::new();
        let occ = queue.occupancy();
        assert_eq!(occ.len(), 3);
        assert_eq!(occ[0].active, 0);
        assert_eq!(occ[0].capacity, 1);
        assert_eq!(occ[1].capacity, 2);
        assert_eq!(occ[2].capacity, 3);
    }

    #[tokio::test]
    async fn test_try_submit_when_full() {
        let queue = CommandQueue::with_capacities(1, 1, 1);

        // Acquire the main permit
        let sem = queue.main_sem.clone();
        let _permit = sem.acquire().await.unwrap();

        // try_submit should return None since lane is full
        let result = queue.try_submit(Lane::Main, async { 42 }).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_custom_capacities() {
        let queue = CommandQueue::with_capacities(2, 4, 6);
        let occ = queue.occupancy();
        assert_eq!(occ[0].capacity, 2);
        assert_eq!(occ[1].capacity, 4);
        assert_eq!(occ[2].capacity, 6);
    }

    #[test]
    fn test_classify_shell_command_read_only() {
        assert_eq!(classify_shell_command("git status"), ShellCommandClass::ReadOnly);
        assert_eq!(classify_shell_command("rg TODO"), ShellCommandClass::ReadOnly);
    }

    #[test]
    fn test_classify_shell_command_mutating() {
        assert_eq!(
            classify_shell_command("git checkout -b feature/wave3"),
            ShellCommandClass::Mutating
        );
        assert_eq!(classify_shell_command("cargo build"), ShellCommandClass::Mutating);
    }

    #[test]
    fn test_shell_command_warning_destructive() {
        let msg = shell_command_warning("rm -rf /tmp/demo", ShellCommandClass::Mutating)
            .expect("destructive warning expected");
        assert!(msg.contains("destructive"));
    }

    #[test]
    fn test_shell_command_warning_anti_stall() {
        let msg = shell_command_warning("sleep 60", ShellCommandClass::Mutating)
            .expect("stall warning expected");
        assert!(msg.contains("Long blocking sleep"));
    }
}
