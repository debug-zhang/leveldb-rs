// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use std::ptr;
use std::mem;
use std::rc::Rc;
use std::cell::RefCell;

const BLOCK_SIZE: usize = 4096;

/// Similar to "Arena" in leveldb C++.
/// Note that even though methods of this require receiver to be unique self
/// reference, the returned values are raw pointers, and thus do not "hold" the
/// self reference. We could change the receiver type to shared reference.
/// However, it makes the code more complex and may require more `try_borrow`
/// calls, which carry certain costs.
/// Therefore, the suggested way is to create `ArenaRef` and use interior
/// mutability.
pub type ArenaRef = Rc<RefCell<Arena>>;

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
        if bytes > BLOCK_SIZE / 4 {
            // Object is more than a quarter of our block size.
            // Allocate it separately to avoid wasting too much space in leftover bytes.
            return self.allocate_new_block(bytes);
        }

        // We waste the remaining space in the current block.
        self.alloc_ptr = self.allocate_new_block(BLOCK_SIZE);
        self.alloc_bytes_remaining = BLOCK_SIZE;

        let result = self.alloc_ptr;
        unsafe {
            self.alloc_ptr = self.alloc_ptr.offset(bytes as isize);
            self.alloc_bytes_remaining -= bytes;
        }
        result
    }

    fn allocate_new_block(&mut self, block_bytes: usize) -> *mut u8 {
        let mut buf: Vec<u8> = Vec::with_capacity(block_bytes);
        unsafe {
            buf.set_len(block_bytes);
            ptr::write_bytes(buf.as_mut_ptr(), 0, block_bytes);
        }

        let result = buf.as_mut_ptr();
        self.blocks.push(buf);
        self.memory_usage = self.memory_usage + block_bytes + mem::size_of::<usize>();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::Arena;
    use crate::util::random::Random;
    use std::slice;
    use crate::util::arena::BLOCK_SIZE;

    #[test]
    fn empty() {
        let arena = Arena::new();
        assert!(arena.alloc_ptr.is_null());
        assert_eq!(arena.alloc_bytes_remaining, 0);
        assert_eq!(arena.memory_usage(), 0);
    }

    #[test]
    fn aligned() {
        let mut arena = Arena::new();
        let ptr_size = std::mem::size_of::<usize>();
        assert!(ptr_size > 1);

        let _ = arena.allocate_fallback(1);
        let _ = arena.allocate_aligned(512);
        assert_eq!(arena.alloc_ptr.is_null(), false);
        assert_eq!(arena.alloc_bytes_remaining, BLOCK_SIZE - 512 - ptr_size);
        // assert_eq!(arena.memory_usage(), 0);
    }

    #[test]
    fn simple() {
        const N: u32 = 100000;
        let mut arena = Arena::new();
        let rnd = Random::new(301);
        let mut bytes: usize = 0;

        for i in 0..N {
            let mut s = if i % (N / 10) == 0 {
                i
            } else if rnd.one_in(4000) {
                rnd.uniform(6000)
            } else if rnd.one_in(10) {
                rnd.uniform(100)
            } else {
                rnd.uniform(20)
            };
            if s == 0 {
                // Our arena disallows size 0 allocations.
                s = 1;
            }
            let s = s as usize;

            let r = if rnd.one_in(10) {
                arena.allocate_aligned(s)
            } else {
                arena.allocate(s)
            };

            unsafe {
                let slice = slice::from_raw_parts_mut(r, s);
                for b in 0..s {
                    // Fill the "i"th allocation with a known bit pattern
                    slice[b] = (i % 256) as u8;
                }
            }
            bytes += s;
            assert!(arena.memory_usage() >= bytes);
            if i > N / 10 {
                assert!(arena.memory_usage() <= (bytes as f32 * 1.10) as usize);
            }

            unsafe {
                let slice = slice::from_raw_parts_mut(r, s);
                for b in 0..s {
                    // Check the "i"th allocation for the known bit pattern
                    assert_eq!(slice[b] & 0xff, (i % 256) as u8);
                }
            }
        }
    }
}