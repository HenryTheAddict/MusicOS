use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::{
    DesktopSection, ImportProgress, OperationTask, SyncProgress, TaskId, TaskKind, TaskPhase,
    TaskProgress, TaskStatus,
};

#[derive(Debug, Default)]
pub struct TaskCenter {
    tasks: Vec<OperationTask>,
    next_id: u64,
}

impl TaskCenter {
    pub const MAX_TASKS: usize = 50;

    pub fn start(
        &mut self,
        kind: TaskKind,
        title: impl Into<String>,
        origin: Option<DesktopSection>,
        cancellable: bool,
    ) -> TaskId {
        self.next_id = self.next_id.saturating_add(1).max(1);
        let id = TaskId(self.next_id);
        self.tasks.insert(
            0,
            OperationTask {
                id,
                kind,
                title: title.into(),
                phase: TaskPhase::Queued,
                status: TaskStatus::Running,
                progress: TaskProgress::Indeterminate,
                current_item: None,
                started_at_ms: now_ms(),
                completed_at_ms: None,
                cancellable,
                retryable: false,
                summary: None,
                error: None,
                origin,
                log: Vec::new(),
            },
        );
        self.trim();
        id
    }

    pub fn update_import(&mut self, id: TaskId, progress: &ImportProgress) {
        let Some(task) = self.task_mut(id) else {
            return;
        };
        task.phase = if progress.processed == 0 {
            TaskPhase::Discovering
        } else {
            TaskPhase::Processing
        };
        task.progress = TaskProgress::Items {
            current: progress.processed as u64,
            total: progress.found as u64,
        };
        task.current_item = non_empty(&progress.current);
        task.status = status(progress.running, progress.cancel_requested, progress.failed);
        if !progress.running {
            task.completed_at_ms = Some(now_ms());
            task.cancellable = false;
            task.retryable = progress.failed > 0;
            task.summary = Some(format!(
                "{} imported · {} duplicates · {} failed",
                progress.imported, progress.duplicates, progress.failed
            ));
        }
        for error in &progress.errors {
            if !task.log.contains(error) {
                task.push_log(error);
            }
        }
    }

    pub fn update_sync(&mut self, id: TaskId, progress: &SyncProgress) {
        let Some(task) = self.task_mut(id) else {
            return;
        };
        let current = progress.uploaded + progress.skipped + progress.failed;
        task.phase = if progress.transcoded > 0 && current == 0 {
            TaskPhase::Transcoding
        } else if progress.running {
            TaskPhase::Uploading
        } else {
            TaskPhase::Verifying
        };
        task.progress = TaskProgress::Items {
            current: current as u64,
            total: progress.found as u64,
        };
        task.current_item = non_empty(&progress.current);
        task.status = status(progress.running, progress.cancel_requested, progress.failed);
        if !progress.running {
            task.completed_at_ms = Some(now_ms());
            task.cancellable = false;
            task.retryable = progress.failed > 0;
            task.summary = Some(format!(
                "{} uploaded · {} skipped · {} failed",
                progress.uploaded, progress.skipped, progress.failed
            ));
        }
        for error in &progress.errors {
            if !task.log.contains(error) {
                task.push_log(error);
            }
        }
    }

    pub fn update_scan(
        &mut self,
        id: TaskId,
        processed: usize,
        discovered: usize,
        current_root: Option<String>,
    ) {
        let Some(task) = self.task_mut(id) else {
            return;
        };
        task.phase = TaskPhase::Processing;
        task.progress = TaskProgress::Items {
            current: processed as u64,
            total: discovered as u64,
        };
        task.current_item = current_root;
        task.status = TaskStatus::Running;
    }

    pub fn update_items(
        &mut self,
        id: TaskId,
        phase: TaskPhase,
        current: usize,
        total: usize,
        current_item: Option<String>,
    ) {
        let Some(task) = self.task_mut(id) else {
            return;
        };
        task.phase = phase;
        task.progress = TaskProgress::Items {
            current: current as u64,
            total: total as u64,
        };
        task.current_item = current_item;
        task.status = TaskStatus::Running;
    }

    pub fn fail(&mut self, id: TaskId, error: impl Into<String>) {
        let Some(task) = self.task_mut(id) else {
            return;
        };
        let error = error.into();
        task.status = TaskStatus::Failed;
        task.completed_at_ms = Some(now_ms());
        task.cancellable = false;
        task.retryable = true;
        task.current_item = None;
        task.error = Some(error.clone());
        task.push_log(error);
    }

    pub fn set_phase(
        &mut self,
        id: TaskId,
        phase: TaskPhase,
        current_item: impl Into<Option<String>>,
    ) {
        if let Some(task) = self.task_mut(id) {
            task.phase = phase;
            task.current_item = current_item.into();
            task.status = TaskStatus::Running;
        }
    }

    pub fn complete(&mut self, id: TaskId, summary: impl Into<String>) {
        if let Some(task) = self.task_mut(id) {
            task.phase = TaskPhase::Finishing;
            task.status = TaskStatus::Completed;
            task.progress = TaskProgress::Items {
                current: 1,
                total: 1,
            };
            task.completed_at_ms = Some(now_ms());
            task.cancellable = false;
            task.retryable = false;
            task.current_item = None;
            task.summary = Some(summary.into());
        }
    }

    pub fn request_cancel(&mut self, id: TaskId) {
        if let Some(task) = self.task_mut(id) {
            task.current_item = Some("Cancellation requested".into());
            task.cancellable = false;
        }
    }

    pub fn cancel(&mut self, id: TaskId, summary: impl Into<String>) {
        if let Some(task) = self.task_mut(id) {
            task.status = TaskStatus::Cancelled;
            task.completed_at_ms = Some(now_ms());
            task.cancellable = false;
            task.retryable = true;
            task.current_item = None;
            task.summary = Some(summary.into());
        }
    }

    pub fn clear_completed(&mut self) {
        self.tasks
            .retain(|task| matches!(task.status, TaskStatus::Queued | TaskStatus::Running));
    }

    pub fn tasks(&self) -> &[OperationTask] {
        &self.tasks
    }

    pub fn running_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Running)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Failed)
            .count()
    }

    pub fn aggregate_fraction(&self) -> Option<f32> {
        let fractions = self
            .tasks
            .iter()
            .filter(|task| task.status == TaskStatus::Running)
            .filter_map(|task| task.progress.fraction())
            .collect::<Vec<_>>();
        (!fractions.is_empty()).then(|| fractions.iter().sum::<f32>() / fractions.len() as f32)
    }

    fn task_mut(&mut self, id: TaskId) -> Option<&mut OperationTask> {
        self.tasks.iter_mut().find(|task| task.id == id)
    }

    fn trim(&mut self) {
        if self.tasks.len() > Self::MAX_TASKS {
            self.tasks.truncate(Self::MAX_TASKS);
        }
    }
}

fn status(running: bool, cancelled: bool, failures: usize) -> TaskStatus {
    if running {
        TaskStatus::Running
    } else if cancelled {
        TaskStatus::Cancelled
    } else if failures > 0 {
        TaskStatus::Failed
    } else {
        TaskStatus::Completed
    }
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_string())
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_progress_drives_a_determinate_task() {
        let mut center = TaskCenter::default();
        let id = center.start(
            TaskKind::LibraryImport,
            "Import music",
            Some(DesktopSection::Music),
            true,
        );
        center.update_import(
            id,
            &ImportProgress {
                running: true,
                found: 10,
                processed: 4,
                current: "song.flac".into(),
                ..Default::default()
            },
        );
        let task = &center.tasks()[0];
        assert_eq!(task.progress.fraction(), Some(0.4));
        assert_eq!(task.current_item.as_deref(), Some("song.flac"));
        assert_eq!(center.running_count(), 1);
    }

    #[test]
    fn clearing_history_preserves_running_tasks() {
        let mut center = TaskCenter::default();
        let running = center.start(TaskKind::LibraryScan, "Scan", None, true);
        let failed = center.start(TaskKind::Update, "Update", None, false);
        center.fail(failed, "missing image");
        center.clear_completed();
        assert_eq!(center.tasks().len(), 1);
        assert_eq!(center.tasks()[0].id, running);
    }
}
