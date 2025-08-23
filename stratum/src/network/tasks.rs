use dashmap::DashMap;
use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};

use tokio::task::{JoinError, JoinHandle};
use tracing::{debug, info};

use crate::error::Result;

/// Unique identifier for tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "task-{}", self.0)
    }
}

/// Task metadata for tracking and debugging
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    pub id: TaskId,
    pub name: String,
    pub description: Option<String>,
    pub created_at: std::time::Instant,
    pub category: TaskCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskCategory {
    Connection,
    Protocol,
    Cleanup,
    Background,
    Network,
}

impl std::fmt::Display for TaskCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskCategory::Connection => write!(f, "connection"),
            TaskCategory::Protocol => write!(f, "protocol"),
            TaskCategory::Cleanup => write!(f, "cleanup"),
            TaskCategory::Background => write!(f, "background"),
            TaskCategory::Network => write!(f, "network"),
        }
    }
}

/// Task handle that wraps Tokio's JoinHandle with metadata
pub struct TaskHandle<T> {
    pub metadata: TaskMetadata,
    pub handle: JoinHandle<T>,
}

impl<T> TaskHandle<T> {
    pub fn new(metadata: TaskMetadata, handle: JoinHandle<T>) -> Self {
        Self { metadata, handle }
    }

    pub fn abort(&self) {
        debug!(
            "Aborting task: {} ({})",
            self.metadata.name, self.metadata.id
        );
        self.handle.abort();
    }

    pub fn is_finished(&self) -> bool {
        self.handle.is_finished()
    }

    pub async fn join(self) -> std::result::Result<T, JoinError> {
        self.handle.await
    }
}

impl<T> std::fmt::Debug for TaskHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskHandle")
            .field("metadata", &self.metadata)
            .field("is_finished", &self.handle.is_finished())
            .finish()
    }
}

/// Centralized async task manager for lifecycle management
#[derive(Debug)]
pub struct TaskManager {
    /// Task counter for generating unique IDs
    task_counter: AtomicU64,
    /// Active tasks indexed by task ID
    tasks: DashMap<TaskId, TaskMetadata>,
    /// Cleanup interval for finished tasks
    cleanup_interval: std::time::Duration,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            task_counter: AtomicU64::new(1),
            tasks: DashMap::new(),
            cleanup_interval: std::time::Duration::from_secs(60),
        }
    }

    pub fn with_cleanup_interval(mut self, interval: std::time::Duration) -> Self {
        self.cleanup_interval = interval;
        self
    }

    /// Generate a new unique task ID
    fn next_task_id(&self) -> TaskId {
        TaskId::new(self.task_counter.fetch_add(1, Ordering::Relaxed))
    }

    /// Spawn a new task with metadata tracking
    pub async fn spawn<F, T>(
        &self,
        name: String,
        category: TaskCategory,
        future: F,
    ) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task_id = self.next_task_id();
        let metadata = TaskMetadata {
            id: task_id,
            name: name.clone(),
            description: None,
            created_at: std::time::Instant::now(),
            category,
        };

        // Track the task
        self.tasks.insert(task_id, metadata.clone());

        // Spawn the task
        let handle = tokio::spawn(future);

        debug!(
            "Spawned task: {} ({}) - category: {}",
            name, task_id, category
        );

        TaskHandle::new(metadata, handle)
    }

    /// Spawn a task with description
    pub async fn spawn_with_description<F, T>(
        &self,
        name: String,
        description: String,
        category: TaskCategory,
        future: F,
    ) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task_id = self.next_task_id();
        let metadata = TaskMetadata {
            id: task_id,
            name: name.clone(),
            description: Some(description.clone()),
            created_at: std::time::Instant::now(),
            category,
        };

        // Track the task
        self.tasks.insert(task_id, metadata.clone());

        // Spawn the task
        let handle = tokio::spawn(future);

        debug!(
            "Spawned task: {} ({}) - {}, category: {}",
            name, task_id, description, category
        );

        TaskHandle::new(metadata, handle)
    }

    /// Remove a task from tracking (call when task completes)
    pub async fn untrack_task(&self, task_id: TaskId) {
        if let Some((_, metadata)) = self.tasks.remove(&task_id) {
            let duration = metadata.created_at.elapsed();
            debug!(
                "Untracking task: {} ({}) - ran for {:?}",
                metadata.name, task_id, duration
            );
        }
    }

    /// Get all active tasks
    pub async fn active_tasks(&self) -> Vec<TaskMetadata> {
        self.tasks
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get task count by category
    pub async fn task_count_by_category(&self) -> HashMap<TaskCategory, usize> {
        let mut counts = HashMap::new();

        for entry in self.tasks.iter() {
            let metadata = entry.value();
            *counts.entry(metadata.category).or_insert(0) += 1;
        }

        counts
    }

    /// Get total active task count
    pub async fn active_task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Shutdown all tasks (abort all active tasks)
    pub async fn shutdown_all(&self) -> Result<()> {
        let task_count = self.tasks.len();

        info!("Shutting down {} active tasks", task_count);

        // Note: This only removes from tracking, actual abort would require storing handles
        // For a complete implementation, we'd need to store the actual JoinHandles

        self.tasks.clear();

        info!("Task shutdown complete");
        Ok(())
    }

    /// Start background cleanup task for finished tasks
    pub async fn start_cleanup_task(&self) -> TaskHandle<()> {
        let task_manager = TaskManager {
            task_counter: AtomicU64::new(self.task_counter.load(Ordering::Relaxed)),
            tasks: DashMap::new(),
            cleanup_interval: self.cleanup_interval,
        };

        let cleanup_interval = self.cleanup_interval;

        self.spawn(
            "task-cleanup".to_string(),
            TaskCategory::Background,
            async move {
                let mut interval = tokio::time::interval(cleanup_interval);

                loop {
                    interval.tick().await;

                    // Cleanup logic would go here
                    // For now, just log periodically
                    let count = task_manager.active_task_count().await;
                    if count > 0 {
                        debug!("Task cleanup tick - {} active tasks", count);
                    }
                }
            },
        )
        .await
    }

    /// Get task statistics
    pub async fn get_statistics(&self) -> TaskManagerStatistics {
        let mut stats = TaskManagerStatistics::default();

        stats.total_active_tasks = self.tasks.len();

        for entry in self.tasks.iter() {
            let metadata = entry.value();
            let age = metadata.created_at.elapsed();

            match metadata.category {
                TaskCategory::Connection => stats.connection_tasks += 1,
                TaskCategory::Protocol => stats.protocol_tasks += 1,
                TaskCategory::Cleanup => stats.cleanup_tasks += 1,
                TaskCategory::Background => stats.background_tasks += 1,
                TaskCategory::Network => stats.network_tasks += 1,
            }

            if age > stats.oldest_task_age {
                stats.oldest_task_age = age;
            }

            if stats.newest_task_age.is_zero() || age < stats.newest_task_age {
                stats.newest_task_age = age;
            }
        }

        stats
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the task manager
#[derive(Debug, Clone)]
pub struct TaskManagerStatistics {
    pub total_active_tasks: usize,
    pub connection_tasks: usize,
    pub protocol_tasks: usize,
    pub cleanup_tasks: usize,
    pub background_tasks: usize,
    pub network_tasks: usize,
    pub oldest_task_age: std::time::Duration,
    pub newest_task_age: std::time::Duration,
}

impl Default for TaskManagerStatistics {
    fn default() -> Self {
        Self {
            total_active_tasks: 0,
            connection_tasks: 0,
            protocol_tasks: 0,
            cleanup_tasks: 0,
            background_tasks: 0,
            network_tasks: 0,
            oldest_task_age: std::time::Duration::ZERO,
            newest_task_age: std::time::Duration::ZERO,
        }
    }
}

impl std::fmt::Display for TaskManagerStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Tasks: {} total (conn: {}, proto: {}, net: {}, bg: {}, cleanup: {}), oldest: {:?}",
            self.total_active_tasks,
            self.connection_tasks,
            self.protocol_tasks,
            self.network_tasks,
            self.background_tasks,
            self.cleanup_tasks,
            self.oldest_task_age
        )
    }
}

/// Helper macro for spawning tasks with automatic tracking
#[macro_export]
macro_rules! spawn_task {
    ($task_manager:expr, $name:expr, $category:expr, $future:expr) => {{
        $task_manager
            .spawn($name.to_string(), $category, $future)
            .await
    }};
}

/// Helper macro for spawning tasks with description
#[macro_export]
macro_rules! spawn_task_with_description {
    ($task_manager:expr, $name:expr, $description:expr, $category:expr, $future:expr) => {{
        $task_manager
            .spawn_with_description(
                $name.to_string(),
                $description.to_string(),
                $category,
                $future,
            )
            .await
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_basic() {
        let manager = TaskManager::new();

        // Test task spawning
        let handle = manager
            .spawn("test-task".to_string(), TaskCategory::Background, async {
                42
            })
            .await;

        assert_eq!(handle.metadata.name, "test-task");
        assert_eq!(
            handle.metadata.category as u8,
            TaskCategory::Background as u8
        );

        // Test task completion
        let result = handle.join().await.unwrap();
        assert_eq!(result, 42);

        // Test statistics
        let stats = manager.get_statistics().await;
        assert_eq!(stats.total_active_tasks, 1); // Still tracked until untracked
    }

    #[tokio::test]
    async fn test_task_categories() {
        let manager = TaskManager::new();

        // Spawn tasks of different categories
        let _handle1 = manager
            .spawn("conn-1".to_string(), TaskCategory::Connection, async {})
            .await;
        let _handle2 = manager
            .spawn("proto-1".to_string(), TaskCategory::Protocol, async {})
            .await;
        let _handle3 = manager
            .spawn("net-1".to_string(), TaskCategory::Network, async {})
            .await;

        let counts = manager.task_count_by_category().await;
        assert_eq!(counts.get(&TaskCategory::Connection), Some(&1));
        assert_eq!(counts.get(&TaskCategory::Protocol), Some(&1));
        assert_eq!(counts.get(&TaskCategory::Network), Some(&1));
    }
}
