//! Process management syscalls

use crate::loader::get_app_data_by_name;
use crate::mm::{translated_refmut, translated_str, translated_large_type, copy_type_into_bufs, mmap, munmap, VirtAddr};
use crate::task::{
    add_task, current_task, current_user_token, exit_current_and_run_next,
    suspend_current_and_run_next, TaskStatus, set_priority,
};
use crate::timer::get_time_us;
use alloc::sync::Arc;
use crate::config::MAX_SYSCALL_NUM;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

#[derive(Clone, Copy)]
pub struct TaskInfo {
    pub status: TaskStatus,
    pub syscall_times: [u32; MAX_SYSCALL_NUM],
    pub time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    debug!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().pid.0 as isize
}

/// Syscall Fork which returns 0 for child process and child_pid for parent process
pub fn sys_fork() -> isize {
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

/// Syscall Exec which accepts the elf path
pub fn sys_exec(path: *const u8) -> isize {
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = current_task().unwrap();
    // find a child process

    // ---- access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB lock exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after removing from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child TCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB lock automatically
}

// YOUR JOB: 引入虚地址后重写 sys_get_time
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    let us = get_time_us();
    let bufs = translated_large_type::<TimeVal>(current_user_token(), ts);
    unsafe {
        copy_type_into_bufs::<TimeVal>(
            &TimeVal {
                sec: us / 1_000_000,
                usec: us % 1_000_000,
            },
            bufs
        );
    }
    0
}
// pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
//     let _us = get_time_us();
//     // unsafe {
//     //     *ts = TimeVal {
//     //         sec: us / 1_000_000,
//     //         usec: us % 1_000_000,
//     //     };
//     // }
//     translated_assign_ptr(
//         current_user_token(),
//         _ts,
//         TimeVal {
//             sec: _us / 1_000_000,
//             usec: _us % 1_000_000,
//         }
//     );
//     0
// }
// pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
//     let _us = get_time_us();
//     let pa_ts = translate_va_to_pa(current_user_token(), (_ts as usize).into()).unwrap().0;
//     unsafe {
//         *(pa_ts as *mut TimeVal) = TimeVal {
//             sec: _us / 1_000_000,
//             usec: _us % 1_000_000,
//         };
//     }
//     0
// }




// YOUR JOB: 引入虚地址后重写 sys_task_info
pub fn sys_task_info(ti: *mut TaskInfo) -> isize {
    // -1
    // let pa_ti = translate_va_to_pa(current_user_token(), (ti as usize).into()).unwrap().0;

    // crate::task::get_task_info(pa_ti as *mut TaskInfo);
    // 0
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    let mut ti_tmp = TaskInfo {
        status: inner.task_status,
        syscall_times: [0; MAX_SYSCALL_NUM],
        time: (get_time_us()-inner.start_time)/1000,
    };
    ti_tmp.syscall_times.clone_from_slice(&inner.syscall_times);
    let bufs = translated_large_type::<TaskInfo>(token, ti);
    unsafe{ copy_type_into_bufs::<TaskInfo>(&ti_tmp, bufs); };
    0
}

// pub fn increase_syscall_time(syscall_number: usize) {
//     crate::task::increase_syscall_time(syscall_number);
// }

// YOUR JOB: 实现sys_set_priority，为任务添加优先级
pub fn sys_set_priority(prio: isize) -> isize {
    // -1
    // if prio < 2 {
    //     return -1;
    // } else {
    //     current_task().unwrap().set_priority(prio)
    // }
    // set_task_priority(prio as usize);
    let current_task = current_task().unwrap();
    if set_priority(&current_task, prio) == 0 {
        prio
    } else {
        -1
    }
}

// YOUR JOB: 扩展内核以实现 sys_mmap 和 sys_munmap
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // -1
    let vstart = VirtAddr::from(start);
    if ! vstart.aligned() || port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    
    let vend = VirtAddr::from(start+len);
    mmap(vstart, vend, port)
    // mmap_in_current_memory_set(start, len, port)
}

pub fn sys_munmap(start: usize, len: usize) -> isize {
    // -1
    let vstart = VirtAddr::from(start);
    if ! vstart.aligned() {
        return -1;
    }
    if len == 0 {
        return 0;
    }

    let vend = VirtAddr::from(usize::from(vstart)+len);
    munmap(vstart, vend)
    // munmap_in_current_memory_set(start, len)
}

//
// YOUR JOB: 实现 sys_spawn 系统调用
// ALERT: 注意在实现 SPAWN 时不需要复制父进程地址空间，SPAWN != FORK + EXEC 
pub fn sys_spawn(path: *const u8) -> isize {
    // -1
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        // let new_task: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(data));
        // let mut new_inner = new_task.inner_exclusive_access();
        // let parent = current_task().unwrap();
        // let mut parent_inner = parent.inner_exclusive_access();
        // new_inner.parent = Some(Arc::downgrade(&parent));
        // parent_inner.children.push(new_task.clone());
        // drop(new_inner);
        // drop(parent_inner);
        let current_task = current_task().unwrap();
        let new_task = current_task.spawn(data);
        let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
        trap_cx.x[10] = 0;
        let new_pid = new_task.pid.0;
        add_task(new_task);
        new_pid as isize
    } else {
        -1
    }
}
