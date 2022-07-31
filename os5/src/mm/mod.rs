//! Memory management implementation
//!
//! SV39 page-based virtual-memory architecture for RV64 systems, and
//! everything about memory management, like frame allocator, page table,
//! map area and memory set, is implemented here.
//!
//! Every task or process has a memory_set to control its virtual memory.


mod address;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod page_table;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use address::{StepByOne, VPNRange};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, translated_refmut, translated_str, PageTableEntry, copy_type_into_bufs, translated_large_type};
use page_table::{PTEFlags, PageTable};
use crate::task::current_task;

/// initiate heap allocator, frame allocator and kernel space
pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_allocator();
    KERNEL_SPACE.exclusive_access().activate();
}

pub fn mmap(
        start_va: VirtAddr,
        end_va: VirtAddr,
        port: usize
    ) -> isize {
    let task = current_task().unwrap();
    let mut cur_task = task.inner_exclusive_access();
    let mem_set = &mut cur_task.memory_set;
    let end_va = end_va.ceil().into();
    if mem_set.has_conflict_with_range(start_va, end_va) {
        return -1;
    }
    let mut perm = MapPermission::U;
    if (port & (1 << 0)) != 0 {
        perm |= MapPermission::R;
    }
    if (port & (1 << 1)) != 0 {
        perm |= MapPermission::W;
    }
    if (port & (1 << 2)) != 0 {
        perm |= MapPermission::X;
    }
    mem_set.insert_framed_area(
        start_va,
        end_va,
        perm
    );
    info!("[PID {}] user mmap: [{:#x}, {:#x}]", task.pid.0, usize::from(start_va), usize::from(end_va));
    0
}

pub fn munmap(
        start_va: VirtAddr,
        end_va: VirtAddr,
    ) -> isize {
    let task = current_task().unwrap();
    let mut cur_task = task.inner_exclusive_access();
    let mem_set = &mut cur_task.memory_set;
    let start_vn = start_va.floor();
    let end_vn = end_va.ceil();
    let ret = mem_set.unmap_area_exact_range(start_vn, end_vn);
    info!("[PID {}] user munmap: [{:#x}, {:#x}]", task.pid.0, usize::from(start_vn), usize::from(end_vn));
    ret
}