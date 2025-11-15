//!Implementation of [`TaskManager`]
use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        if self.ready_queue.is_empty() {
            return None;
        }
        let mut min_idx = 0;
        let mut min_stride = {
            let inner = self.ready_queue[0].inner_exclusive_access();
            inner.stride
        };
        for i in 1..self.ready_queue.len() {
            let stride_i = {
                let inner = self.ready_queue[i].inner_exclusive_access();
                inner.stride
            };
            if stride_i < min_stride {
                min_stride = stride_i;
                min_idx = i;
            }
        }
        // min_stride > 阈值 全部减去一个值
        if min_stride > BIG_STRIDE {
            self.ready_queue
                .iter_mut()
                .for_each(|tcb| tcb.inner_exclusive_access().stride -= min_stride);
        }

        // remove chosen task from ready_queue and return it
        self.ready_queue.remove(min_idx)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
