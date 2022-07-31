//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use super::TaskControlBlock;
use crate::sync::UPSafeCell;
use alloc::collections::{VecDeque, BTreeMap};
use alloc::sync::Arc;
use lazy_static::*;

// TaskManager 进行了一次减负，把当前运行进程的信息全部放入到了Processor结构，减负后的结构为：
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    btmap: BTreeMap<usize, usize>,
    // ready_queue: BinaryHeap<Arc<TaskControlBlock>>
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            btmap: BTreeMap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push_back(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        // return self.ready_queue.pop_front();
        if self.ready_queue.is_empty() {
            return None;
        }
        let mut min_stride = self.ready_queue.get(0 as usize).unwrap().inner_exclusive_access().task_stride;
        let mut index = 0;
        for (i, task) in self.ready_queue.iter().enumerate() {
            let inner = task.inner_exclusive_access();
            let gap: i8 = (inner.task_stride - min_stride) as i8;
            if gap < 0 {
                min_stride = inner.task_stride;
                index = i;
            }
        }
        let pid = self.ready_queue.get(index).unwrap().pid.0;
        // only for ch5_stride_test
        match self.btmap.get(&pid) {
            Some(item) => {
                self.btmap.insert(pid, item + 1);
            }
            None => {
                self.btmap.insert(pid, 0);
            }
        }
        if self.btmap.len() < 411 {
            println!("DEBUG : {:?}", self.btmap);
        }
        return self.ready_queue.remove(index);
    }
}

// 实例化
lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

pub fn add_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.exclusive_access().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    TASK_MANAGER.exclusive_access().fetch()
}
