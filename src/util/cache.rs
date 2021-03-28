// Copyright (c) 2021, storagezhang <storagezhang@outlook.com>. All rights reserved.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file. See the AUTHORS file for names of contributors.

use crate::util::slice::Slice;
use std::rc::Rc;
use std::cell::RefCell;
use std::fmt::Debug;
use std::ptr;
use std::collections::HashMap;
use std::sync::Mutex;
use std::ops::DerefMut;
use std::borrow::{Borrow, BorrowMut};

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
              deleter: Box<dyn FnMut(&Slice, &T)>) -> HandlePtr<T>;

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

impl<T> LRUHandle<T> where
    T: Default + Debug {
    fn new(value: T, charge: usize, deleter: Box<dyn FnMut(&Slice, &T)>,
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
        assert_ne!(self.next, self.prev);
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

/// TODO
type HandleTable<T> = HashMap<Slice, LRUHandlePtr<T>>;

/// mutex protects the following state.
struct MutexData<T: Default + Debug> {
    usage: usize,

    // Dummy head of LRU list.
    // lru.prev is newest entry, lru.next is oldest entry.
    // Entries have refs==1 and in_cache==true.
    lru: *mut LRUHandle<T>,

    // Dummy head of in-use list.
    // Entries are in use by clients, and have refs >= 2 and in_cache==true.
    in_use: *mut LRUHandle<T>,
    table: HandleTable<T>,
}

/// A single shard of sharded cache.
struct LRUCache<T: Default + Debug + 'static> {
    // Initialized before use.
    capacity: usize,

    // mutex protects the following state.
    mutex: Mutex<MutexData<T>>,
}

impl<T> LRUCache<T> where
    T: Default + Debug + 'static {
    fn new(capacity: usize) -> Self {
        let mutex_data = MutexData {
            usage: 0,
            lru: Self::create_dummy_node(),
            in_use: Self::create_dummy_node(),
            table: HashMap::default(),
        };

        Self {
            capacity,
            mutex: Mutex::new(mutex_data),
        }
    }

    fn lru_remove(e: *mut LRUHandle<T>) {
        unsafe {
            (*(*e).next).prev = (*e).prev;
            (*(*e).prev).next = (*e).next;
        }
    }

    fn lru_append(list: *mut LRUHandle<T>, e: *mut LRUHandle<T>) {
        unsafe {
            // Make `e` newest entry by inserting just before list
            (*e).next = list;
            (*e).prev = (*list).prev;
            (*(*e).prev).next = e;
            (*(*e).next).prev = e;
        }
    }

    fn inc_ref(list: *mut LRUHandle<T>, e: &LRUHandlePtr<T>) {
        // If on lru list, move to in_use list.
        if Rc::strong_count(e) == 1 {
            let p = e.borrow().deref_mut().as_ptr();
            Self::lru_remove(p);
            Self::lru_append(list, p);
        }
    }

    fn dec_ref(list: *mut LRUHandle<T>, e: LRUHandlePtr<T>) {
        let c = Rc::strong_count(&e);
        assert!(c > 0);
        if c == 2 {
            // Deallocate.
            let p = e.borrow().deref_mut().as_ptr();
            Self::lru_remove(p);
            Self::lru_append(list, p);
        }
    }

    fn finish_erase(mutex_data: &mut MutexData<T>, mut e: LRUHandlePtr<T>) {
        mutex_data.usage -= e.borrow().charge;
        Self::lru_remove(e.borrow().deref_mut().as_ptr());
        Self::dec_ref(mutex_data.lru, e);
    }

    fn create_dummy_node() -> *mut LRUHandle<T> {
        unsafe {
            let n = Box::into_raw(Box::new(LRUHandle::default()));
            (*n).next = n;
            (*n).prev = n;
            n
        }
    }

    fn drop_dummy_node(n: *mut LRUHandle<T>) {
        assert!(!n.is_null());
        unsafe {
            let _ = Box::from_raw(n);
        }
    }
}

impl<T> Drop for LRUCache<T> where
    T: Default + Debug + 'static {
    fn drop(&mut self) {
        let mutex_data = self.mutex.lock().unwrap();
        Self::drop_dummy_node(mutex_data.lru);
        Self::drop_dummy_node(mutex_data.in_use);
    }
}

impl<T: Default + Debug + 'static> Cache<T> for LRUCache<T> {
    fn insert(&mut self, key: Slice, value: T, charge: usize,
              deleter: Box<dyn FnMut(&Slice, &T)>) -> HandlePtr<T> {
        let mut mutex_data = self.mutex.lock().unwrap();

        let key_data = Vec::from(key.slice_data()).into_boxed_slice();
        let mut e = LRUHandle::new(value, charge, deleter, key_data);

        let r = if self.capacity > 0 {
            // for the cache's reference.
            let r = Rc::new(RefCell::new(e));
            Self::lru_append(mutex_data.in_use, r.clone().borrow_mut().as_ptr());
            mutex_data.usage += charge;
            if let Some(old) = mutex_data.table.insert(key, r.clone()) {
                Self::finish_erase(&mut mutex_data, old);
            };
            r
        } else {
            // don't cache. (capacity_==0 is supported and turns off caching.)
            // next is read by key() in an assert, so it must be initialized
            e.next = ptr::null_mut();
            Rc::new(RefCell::new(e))
        };

        let lru = mutex_data.lru;
        unsafe {
            while mutex_data.usage > self.capacity && (*lru).next != lru {
                let old = (*lru).next;
                if let Some(old) = mutex_data.table.remove(&(*old).key()) {
                    assert_eq!(Rc::strong_count(&old), 1);
                    Self::finish_erase(&mut mutex_data, old);
                }
            }
        }

        r
    }

    fn lookup(&self, key: &Slice) -> Option<HandlePtr<T>> {
        let mutex_data = self.mutex.lock().unwrap();
        match mutex_data.table.get(&key) {
            Some(e) => {
                Self::inc_ref(mutex_data.in_use, e);
                Some(e.clone())
            }
            None => None,
        }
    }

    fn release(&mut self, handle: HandlePtr<T>) {
        let mutex_data = self.mutex.lock().unwrap();
        Self::dec_ref(mutex_data.lru, handle);
    }

    fn value(&self, handle: HandlePtr<T>);

    fn erase(&mut self, key: &Slice);

    fn new_id(&self) -> u64;

    fn prune(&mut self);

    fn total_charge(&self) -> usize;
}