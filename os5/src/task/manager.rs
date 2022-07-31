//! Implementation of [`TaskManager`]
//!
//! It is only used to manage processes and schedule process based on ready queue.
//! Other CPU process monitoring functions are in Processor.


use core::cmp::Ordering;

use super::TaskControlBlock;
use crate::config::BIG_STRIDE;
use crate::sync::UPSafeCell;
// use alloc::collections::{VecDeque, BTreeMap};
use alloc::vec::Vec;
use alloc::sync::Arc;
use lazy_static::*;

// TaskManager 进行了一次减负，把当前运行进程的信息全部放入到了Processor结构，减负后的结构为：
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
    // btmap: BTreeMap<usize, usize>,
    // ready_queue: BinaryHeap<Arc<TaskControlBlock>>
}

// YOUR JOB: FIFO->Stride
/// A simple FIFO scheduler.
impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
            // btmap: BTreeMap::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        // let task_inner = task.inner_exclusive_access();
        // let stride = task_inner.task_stride;
        // drop(task_inner);
        // let len = self.ready_queue.len();
        // for queue in 0..len {
        //     let task1 = self.ready_queue.get_mut(queue).unwrap();
        //     let stride1 = task1.inner_exclusive_access().task_stride;
        //     if stride < stride1 {
        //         self.ready_queue.insert(queue, task);
        //         return
        //     }
        // }
        self.ready_queue.push(task)
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // self.ready_queue.pop_front()
        // return self.ready_queue.pop_front();
        if self.ready_queue.is_empty() {
            return None;
        }
        // let mut min_stride = self.ready_queue.get(0 as usize).unwrap().inner_exclusive_access().task_stride;
        // let mut index = 0;
        // for (i, task) in self.ready_queue.iter().enumerate() {
        //     let inner = task.inner_exclusive_access();
        //     let gap: i8 = (inner.task_stride - min_stride) as i8;
        //     if gap < 0 {
        //         min_stride = inner.task_stride;
        //         index = i;
        //     }
        // }
        // let pid = self.ready_queue.get(index).unwrap().pid.0;
        // only for ch5_stride_test
        // match self.btmap.get(&pid) {
        //     Some(item) => {
        //         self.btmap.insert(pid, item + 1);
        //     }
        //     None => {
        //         self.btmap.insert(pid, 0);
        //     }
        // }
        // if self.btmap.len() < 411 {
        //     println!("DEBUG : {:?}", self.btmap);
        // }
        // return self.ready_queue.remove(index);
        /// 我在这里卡了几个小时，！！！ NONONO
        let mut min_i = 0;
        let mut min_stride = self.ready_queue[0].inner_exclusive_access().task_stride;
        for i in 0..self.ready_queue.len() {
            let stride = self.ready_queue[i].inner_exclusive_access().task_stride;
            if stride < min_stride {
                min_i = i;
                min_stride = stride;
            }
        }
        Some(self.ready_queue.swap_remove(min_i))
    }
}


#[derive(Copy, Clone)]
pub struct Pass(pub u64);

impl Pass {
    pub fn new() -> Self {
        Self(0)
    }
    pub fn step_by_prio(&mut self, priority: isize) {
        let stride = match BIG_STRIDE as u64 / priority as u64 {
            0 => 1,
            o => o,
        };
        self.0 += stride;
    }
}

impl PartialOrd for Pass {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let overflow = self.0.abs_diff(other.0) > BIG_STRIDE as u64 / 2;
        let order = self.0 <= other.0;
        if order ^ overflow {
            Some(Ordering::Less)
        }
        else {
            Some(Ordering::Greater)
        }
    }
}

impl PartialEq for Pass {
    fn eq(&self, _other: &Self) -> bool {
        false
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
    // TASK_MANAGER.exclusive_access().fetch()
    let task = TASK_MANAGER.exclusive_access().fetch()?;
    {
        let mut task_inner = task.inner_exclusive_access();
        let priority = task_inner.task_priority;
        task_inner.task_stride.step_by_prio(priority as isize);
        info!("fetch task with PID {}, pass {}", task.pid.0, task_inner.task_stride.0);
    }
    Some(task)
}

pub fn set_priority(task: &TaskControlBlock, priority: isize) -> isize{
    if priority < 2 {
        -1
    }
    else {
        let mut task_inner = task.inner_exclusive_access();
        task_inner.task_priority = priority as usize;
        0
    }
}
