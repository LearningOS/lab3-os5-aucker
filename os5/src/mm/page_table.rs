//! Implementation of [`PageTableEntry`] and [`PageTable`].

// use super::address::VPNRange;
use super::{frame_alloc, FrameTracker, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum};
use core::mem::size_of;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::slice::from_raw_parts;
use bitflags::*;

bitflags! {
    /// page table entry flags
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<FrameTracker>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    // 临时创建一个专用手动查页表的pagetable，仅有一个从传入的satp token中得到的
    // 多级页表根节点的物理页号，它的frames字段为空，即不控制任何资源
    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let mut idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter_mut().enumerate() {
            let pte = &mut ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTEFlags::V);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &ppn.get_pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    #[allow(unused)]
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }
    #[allow(unused)]
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).copied()
    }
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.clone().floor()).map(|pte| {
            //println!("translate_va:va = {:?}", va);
            let aligned_pa: PhysAddr = pte.ppn().into();
            //println!("translate_va:pa_align = {:?}", aligned_pa);
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
}

/// translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(token: usize, ptr: *const u8, len: usize) -> Vec<&'static mut [u8]> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.get_bytes_array()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    v
}

pub fn translated_str(token: usize, ptr: *const u8) -> String {
    let page_table = PageTable::from_token(token);
    let mut string = String::new();
    let mut va = ptr as usize;
    loop {
        let ch: u8 = *(page_table
            .translate_va(VirtAddr::from(va))
            .unwrap()
            .get_mut());
        if ch == 0 {
            break;
        } else {
            string.push(ch as char);
            va += 1;
        }
    }
    string
}

pub fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    //println!("into translated_refmut!");
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    //println!("translated_refmut: before translate_va");
    page_table
        .translate_va(VirtAddr::from(va))
        .unwrap()
        .get_mut()
}


// give bare pointer value
// pub fn translated_assign_ptr<T: Debug>(token: usize, ptr: *mut T, value: T) {
//     let page_table = PageTable::from_token(token);
//     let va = VirtAddr::from(ptr as usize);
//     let vpn = va.floor();
//     let offset = va.page_offset();
//     let ppn = page_table.translate(vpn).unwrap().ppn();
//     let pa: PhysAddr = (usize::from(PhysAddr::from(ppn)) + offset).into();
//     unsafe {
//         let ptr_pa = (pa.0 as *mut T).as_mut().unwrap();
//         *ptr_pa = value;
//     }
// }

// pub fn translate_va_to_pa(token: usize, va: VirtAddr) -> Option<PhysAddr> {
//     let page_table = PageTable::from_token(token);
//     page_table.find_pte(va.clone().floor())
//         .map(|pte| {
//             let aligned_pa: PhysAddr = pte.ppn().into();
//             let offset = va.page_offset();
//             let aligned_pa_usize: usize = aligned_pa.into();
//             (aligned_pa_usize + offset).into()
//         })
// }

// pub fn has_mapped(token: usize, start: usize, len: usize) -> bool {
//     let start_vpn = VirtAddr::from(start).floor();
//     let end_vpn = VirtAddr::from(start + len).ceil();
//     let page_table = PageTable::from_token(token);
//     for vpn in VPNRange::new(start_vpn, end_vpn) {
//         if let Some(x) = page_table.translate(vpn) {
//             if x.is_valid() == true {
//                 return false;
//             }
//         }
//     }
//     true
// }

// pub fn has_unmapped(token: usize, start: usize, len: usize) -> bool {
//     let start_vpn = VirtAddr::from(start).floor();
//     let end_vpn = VirtAddr::from(start + len).ceil();
//     let page_table = PageTable::from_token(token);
//     for vpn in VPNRange::new(start_vpn, end_vpn) {
//         match page_table.translate(vpn) {
//             Some(x) => {
//                 if x.is_valid() == false {
//                     return true;
//                 }
//             }
//             None => {
//                 return true;
//             }
//         }
//     }
//     false
// }

/// for type so large that spans multiple pages
/// or even trickier, small type that cross border between 2 pages, unlikely
pub fn translated_large_type<T>(token: usize, ptr: *const T) -> Vec<& 'static mut [u8]> {
    let ptr = ptr as *const u8;
    let size = size_of::<T>();
    translated_byte_buffer(token, ptr, size)
}

pub unsafe fn copy_type_into_bufs<T>(value: &T, buffers: Vec<&mut [u8]>) {
    let value = from_raw_parts(value as *const T as *const u8, size_of::<T>());
    let mut offset = 0;
    for buffer in buffers {
        let dst_len = buffer.len();    
        buffer.copy_from_slice(&value[offset..offset+dst_len]);
        offset += dst_len;
    }
}