// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use std::ptr;
use std::mem;

const BLOCK_SIZE: usize = 4096;

pub struct Arena {
    // Allocation state
    alloc_ptr: *mut u8,
    alloc_bytes_remaining: usize,

    // Vector of new allocated memory blocks
    blocks: Vec<Vec<u8>>,

    // Total memory usage of the arena.
    //
    // TODO: This member is accessed via atomics, but the others are accessed without any locking.
    //       Is this OK?
    memory_usage: usize,
}

impl Arena {
    pub fn new() -> Self {
        Self {
            alloc_ptr: ptr::null_mut(),
            alloc_bytes_remaining: 0,
            blocks: Vec::new(),
            memory_usage: 0,
        }
    }

    /// Return a pointer to a newly byte slice with length `bytes`.
    #[inline]
    pub fn allocate(&mut self, bytes: usize) -> *mut u8 {
        // The semantics of what to return are a bit messy if we allow
        // 0-byte allocations, so we disallow them here (we don't need
        // them for our internal use).
        assert!(bytes > 0);
        if bytes <= self.alloc_bytes_remaining {
            let result = self.alloc_ptr;
            unsafe {
                self.alloc_ptr = self.alloc_ptr.offset(bytes as isize);
                self.alloc_bytes_remaining -= bytes;
            }
            result
        } else {
            self.allocate_fallback(bytes)
        }
    }

    /// Return a pointer aligned to a newly byte slice with length `bytes`.
    pub fn allocate_aligned(&mut self, bytes: usize) -> *mut u8 {
        let ptr_size = mem::size_of::<usize>();
        let aligns = if ptr_size > 8 {
            ptr_size
        } else {
            8
        };
        // Pointer size should be a power of 2.
        assert_eq!((aligns & (aligns - 1)), 0);

        let current_mode = (self.alloc_ptr as usize) & (aligns - 1);
        let slop = if current_mode == 0 {
            0
        } else {
            aligns - current_mode
        };
        let needed = bytes + slop;

        let result = if needed <= self.alloc_bytes_remaining {
            unsafe {
                let tmp = self.alloc_ptr.offset(slop as isize);
                self.alloc_ptr = self.alloc_ptr.offset(needed as isize);
                self.alloc_bytes_remaining -= needed;
                tmp
            }
        } else {
            // allocate_fallback always returned aligned memory
            self.allocate_fallback(bytes)
        };

        assert_eq!((result as usize) & (aligns - 1), 0);
        result
    }

    /// Returns an estimate of the total memory usage of data allocated by the arena.
    pub fn memory_usage(&self) -> usize {
        self.memory_usage
    }

    /// TODO
    fn allocate_fallback(&mut self, bytes: usize) -> *mut u8 {
        ptr::null_mut()
    }
}