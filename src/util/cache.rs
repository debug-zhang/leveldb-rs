// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use crate::util::slice::Slice;
use crate::util::hash;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ptr;

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

    /// Release a mapping returned by a previous `insert` or `lookup`.
    /// REQUIRES: `handle` must not have been released yet.
    /// REQUIRES: `handle` must have been returned by a method on this instance.
    fn release(&mut self, handle: HandlePtr<T>);

    /// Return the value encapsulated in a handle returned by a successful `lookup`.
    /// REQUIRES: handle must not have been released yet.
    /// REQUIRES: handle must have been returned by a method on this instance.
    fn value(&self, handle: HandlePtr<T>);

    /// If the cache contains entry for key, erase it.  Note that the
    /// underlying entry will be kept around until all existing handles
    /// to it have been released.
    fn erase(&mut self, key: &Slice);

    /// Return a new numeric id.  May be used by multiple clients who are
    /// sharing the same cache to partition the key space.  Typically the
    /// client will allocate a new id at startup and prepend the id to
    /// its cache keys.
    fn new_id(&self) -> u64;

    /// Remove all cache entries that are not actively in use.  Memory-constrained
    /// applications may wish to call this method to reduce memory usage.
    fn prune(&mut self);

    /// Return an estimate of the combined charges of all elements stored in the cache.
    fn total_charge(&self) -> usize;
}

/// LRU cache implementation
///
/// Cache entries have an "in_cache" boolean indicating whether the cache has a
/// reference on the entry.  The only ways that this can become false without the
/// entry being passed to its "deleter" are via Erase(), via Insert() when
/// an element with a duplicate key is inserted, or on destruction of the cache.
///
/// The cache keeps two linked lists of items in the cache.  All items in the
/// cache are in one list or the other, and never both.  Items still referenced
/// by clients but erased from the cache are in neither list.  The lists are:
/// - in-use:  contains the items currently referenced by clients, in no
///   particular order.  (This list is used for invariant checking.  If we
///   removed the check, elements that would otherwise be on this list could be
///   left as disconnected singleton lists.)
/// - LRU:  contains the items not currently referenced by clients, in LRU order
/// Elements are moved between these lists by the Ref() and Unref() methods,
/// when they detect an element in the cache acquiring or losing its only
/// external reference.


/// An entry is a variable length heap-allocated structure.  Entries
/// are kept in a circular doubly linked list ordered by access time.
struct LRUHandle<T: Default + Debug> {
    value: T,
    deleter: Option<Box<dyn FnMut(&Slice, &T)>>,
    next: *mut LRUHandle<T>,
    prev: *mut LRUHandle<T>,
    charge: usize,
    key_data: Box<[u8]>,
}

type LRUHandlePtr<T> = Rc<RefCell<LRUHandle<T>>>;

impl LRUHandle<T> where
    T: Debug + Debug {
    fn new(value: T, charge: usize, deleter: Box<dyn FnMt((&Slice, &T))>,
           key_data: Box<[u8]>) -> Self {
        Self {
            value,
            deleter: Some(deleter),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            charge,
            key_data,
        }
    }

    fn key(&self) -> Slice {
        assert_ne!(next, self);
        Slice::from(&self.key_data[..])
    }
}

impl<T> Drop for LRUHandle<T> where
    T: Default + Debug {
    fn drop(&mut self) {
        // Only drop for non-dummy nodes with non-empty deleter.
        let key = self.key();
        if let Some(ref mut deleter) = self.deleter {
            (deleter)(&key, &self.value);
        }
    }
}

impl<T> Default for LRUHandle<T> where
    T: Default + Debug {
    fn default() -> Self {
        LRUHandle {
            value: T::default(),
            deleter: None,
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            charge: 0,
            key_data: Vec::new().into_boxed_slice(),
        }
    }
}

impl<T> Handle<T> for LRUHandle<T> where
    T: Default + Debug {
    fn get_value(&self) -> &T {
        &self.value
    }
}