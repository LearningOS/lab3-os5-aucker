//! Implementation of [`Processor`] and Intersection of control flow
//!
//! Here, the continuous operation of user apps in CPU is maintained,
//! the current running state of CPU is recorded,
//! and the replacement and transfer of control flow of different applications are executed.


use super::__switch;
use super::{fetch_task, TaskStatus};
use super::{TaskContext, TaskControlBlock};
// use crate::config::{PAGE_SIZE, BIG_STRIDE};
use crate::sync::UPSafeCell;
// use crate::syscall::process::TaskInfo;
use crate::timer::get_time_us;
use crate::trap::TrapContext;
use alloc::sync::Arc;
use lazy_static::*;
// use crate::mm::{has_mapped, has_unmapped, MapPermission};

/// Processor management structure
/// 用processor结构来管理运行中的进程，Processor代表处理器，这样的抽象更接近进程的本质，也可以更好地应用于多核：
pub struct Processor {
    /// The task currently executing on the current processor
    current: Option<Arc<TaskControlBlock>>,
    /// The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

/// current中存放着当前运行进程的进程控制块，而idle_task_cx则是idle_task的context，idle_task实际上是用来作为进程切换的的中转的
/// 在进程被调度的时候，会先切换到idle_task,再从idle_task切换到next_task. 
impl Processor {
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::zero_init(),
        }
    }
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(|task| Arc::clone(task))
    }
}

// 实例化了Processor作为处理器管理的结构，并且把对其的操作封装成了各种接口：
lazy_static! {
    /// PROCESSOR instance through lazy_static!
    pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe { UPSafeCell::new(Processor::new()) };
}

/// The main part of process execution and scheduling
///
/// Loop fetch_task to get the process that needs to run,
/// and switch the process through __switch
pub fn run_tasks() {
    loop {
        let mut processor = PROCESSOR.exclusive_access();
        if let Some(task) = fetch_task() {
            let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
            // access coming task TCB exclusively
            let mut task_inner = task.inner_exclusive_access();
            let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
            // task_inner.task_status = TaskStatus::Running;
            if task_inner.start_time == 0 {
                task_inner.start_time = get_time_us();
            }
            task_inner.task_status = TaskStatus::Running;
            // task_inner.task_stride += BIG_STRIDE / task_inner.task_priority;
            drop(task_inner);
            // release coming task TCB manually
            processor.current = Some(task);
            // release processor manually
            drop(processor);
            unsafe {
                __switch(idle_task_cx_ptr, next_task_cx_ptr);
            }
        }
    }
}

/// Get current task through take, leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().take_current()
}

/// Get a copy of the current task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    PROCESSOR.exclusive_access().current()
}

/// Get token of the address space of current task
pub fn current_user_token() -> usize {
    let task = current_task().unwrap();
    let token = task.inner_exclusive_access().get_user_token();
    token
}

/// Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .get_trap_cx()
}

// 之前进程主动放弃CPU时，会去主动执行run_next_task()去去切换进程。

/// 当当前进程需要被调度的时候，我们需要使用schedule方法：
/// Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let mut processor = PROCESSOR.exclusive_access();
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    drop(processor);
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}

// pub fn get_task_info(ti: *mut TaskInfo) {
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         unsafe {
//             (*ti).syscall_times = inner.syscall_times;
//             (*ti).status = TaskStatus::Running;
//             (*ti).time = {
//                 let us: usize = get_time_us() - inner.start_time;
//                 let sec = us / 1_000_000;
//                 let usec = us % 1_000_000;
//                 ((sec & 0xffff) * 1000 + usec / 1000) as usize
//             };
//         }
//     }
// }

// pub fn increase_syscall_time(syscall_number: usize) {
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         inner.syscall_times[syscall_number] += 1;
//     }
//     // let mut inner = current_task().unwrap().inner_exclusive_access();
// }

// pub fn mmap(start: usize, len: usize, port: usize) -> isize {
//     if start % PAGE_SIZE != 0 || port & !0x7 != 0 || port & 0x7 == 0 {
//         return -1;
//     }
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         if has_mapped(inner.get_user_token(), start, len) == false {
//             return -1;
//         }
//         inner.memory_set.mmap(start, len, port);
//         // inner.memory_set.insert_framed_area(VirtAddr::from(start), VirtAddr::from(start + len), permission)
//     }
//     0
// }

// pub fn munmap(start: usize, len: usize) -> isize {
//     if start % PAGE_SIZE != 0 {
//         return -1;
//     }
//     if let Some(task) = current_task() {
//         let mut inner = task.inner_exclusive_access();
//         if has_unmapped(inner.get_user_token(), start, len) {
//             return -1;
//         }
//         inner.memory_set.munmap(start, len);
//     }
//     0
// }