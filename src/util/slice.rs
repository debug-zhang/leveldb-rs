// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use std::ptr;
use std::slice;
use std::cmp::Ordering;
use std::ops::Index;
use std::hash::Hash;

extern crate rlibc;

/// Just like Rust's slice, except there's no borrowing.
/// Instead, the user needs to guarantee that the instances of this struct
/// should not live longer than the memory that `data` points to.
#[derive(Clone, Debug, Eq, Hash)]
pub struct Slice {
    data: *const u8,
    size: usize,
}

impl Slice {
    /// Create a slice that refers to data[0,n-1].
    pub fn new(d: *const u8, n: usize) -> Self {
        Self {
            data: d,
            size: n,
        }
    }

    /// Create an empty slice.
    pub fn new_empty() -> Self {
        Self {
            data: ptr::null(),
            size: 0,
        }
    }

    /// Create a slice that refers to the contents of "s".
    pub fn new_from_string(str: String) -> Self {
        Self {
            data: str.as_ptr(),
            size: str.len(),
        }
    }

    /// Return a pointer to the referenced data.
    #[inline]
    pub fn raw_ptr_data(&self) -> *const u8 {
        self.data
    }

    /// Return a slice to the referenced data.
    #[inline]
    pub fn slice_data(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self.data, self.size)
        }
    }

    /// Return the length (in bytes) of the referenced data.
    #[inline]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Return true iff the length of the referenced data is zero.
    #[inline]
    pub fn empty(&self) -> bool {
        self.size == 0
    }

    /// Change this slice to refer to an empty array.
    #[inline]
    pub fn clear(&mut self) {
        self.data = ptr::null();
        self.size = 0;
    }

    /// Drop the first "n" bytes from this slice.
    pub fn remove_prefix(&mut self, n: usize) {
        assert!(n <= self.size);
        unsafe {
            self.data = self.data.offset(n as isize);
        }
        self.size -= n;
    }

    /// Return a string that contains the copy of the referenced data.
    pub fn to_string(&self) -> String {
        unsafe {
            ::std::str::from_utf8_unchecked(self.slice_data())
        }.to_string()
    }

    /// Return true iff "x" is a prefix of "self".
    pub fn starts_with(&self, x: &Slice) -> bool {
        (self.size >= x.size) && unsafe {
            rlibc::memcmp(self.data, x.data, x.size) == 0
        }
    }

    /// Three-way comparison. Returns value:
    ///   `Ordering::Less`    iff `self` < `b`
    ///   `Ordering::Equal`   iff `self` = `b`
    ///   `Ordering::Greater` iff `self` > `b`
    pub fn compare(&self, b: &Slice) -> Ordering {
        let min_len = if self.size < b.size {
            self.size
        } else {
            b.size
        };

        let r = unsafe { rlibc::memcmp(self.data, b.data, min_len) };
        if r == 0 {
            if self.size < b.size {
                Ordering::Less
            } else if self.size > b.size {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        } else if r < 0 {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

impl Index<usize> for Slice {
    type Output = u8;

    /// Return the ith byte in the referenced data.
    /// REQUIRES: index < size()
    fn index(&self, index: usize) -> &u8 {
        assert!(index < self.size);
        unsafe {
            &*self.data.offset(index as isize)
        }
    }
}

impl<'a> From<&'a [u8]> for Slice {
    #[inline]
    fn from(s: &'a [u8]) -> Self {
        Slice::new(s.as_ptr(), s.len())
    }
}

impl<'a> From<&'a Vec<u8>> for Slice {
    #[inline]
    fn from(v: &'a Vec<u8>) -> Self { Slice::new(v[..].as_ptr(), v.len()) }
}

impl<'a> From<&'a str> for Slice {
    #[inline]
    fn from(s: &'a str) -> Self { Slice::new(s.as_ptr(), s.len()) }
}

impl From<String> for Slice {
    #[inline]
    fn from(s: String) -> Self { Slice::new(s.as_ptr(), s.len()) }
}

impl PartialEq for Slice {
    fn eq(&self, other: &Slice) -> bool {
        self.compare(other) == Ordering::Equal
    }
}