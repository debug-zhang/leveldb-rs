// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use crate::util::slice::Slice;
use crate::util::hash;
use std::rc::Rc;
use std::cell::RefCell;

pub trait Handle<T> {
    fn get_value(&self) -> &T;
}

type HandlePtr<T> = Rc<RefCell<dyn Handle<T>>>;

pub trait Cache<T> {
    /// Insert a mapping from key->value into the cache and assign it
    /// the specified charge against the total cache capacity.
    ///
    /// Returns a handle that corresponds to the mapping.  The caller must call
    /// `release(handle)` when the returned mapping is no longer needed.
    ///
    /// When the inserted entry is no longer needed, the key and
    /// value will be passed to `deleter`.
    fn insert(&mut self, key: Slice, value: T, charge: usize,
              deleter: Box<dyn FnMut(&Slice, &T)>) -> dyn Handle<T>;

    /// If the cache has no mapping for "key", returns nullptr.
    ///
    /// Else return a handle that corresponds to the mapping.  The caller
    /// must call `release(handle)` when the returned mapping is no
    /// longer needed.
    fn lookup(&self, key: &Slice) -> Option<HandlePtr<T>>;

    /// Release a mapping returned by a previous `insert` or `lookup()`.
    /// REQUIRES: `handle` must not have been released yet.
    /// REQUIRES: `handle` must have been returned by a method on this instance.
    fn release(&mut self, handle: HandlePtr<T>);


}