use core::cell::UnsafeCell;
use bytemuck::*;
use fxhash::hash64;

pub struct SharedHashTable<T: Clone + Sized + Send + Sync + NoUninit> {
    inner: Box<[UnsafeCell<TableEntry<T>>]>,
}

#[repr(packed)]
#[derive(Default, Clone)]
pub struct TableEntry<T: Clone + Sized + Send + Sync + NoUninit> {
    key: u64,
    hash: u64,
    value: T,
}

unsafe impl<T: Default + Clone + Sized + Send + Sync + NoUninit> Sync for SharedHashTable<T> {}

impl<T: Default + Clone + Sized + Send + Sync + NoUninit> SharedHashTable<T> {
    pub const fn entry_size() -> usize { core::mem::size_of::<TableEntry<T>>() }

    pub fn new(size: usize) -> Self {
        let mut inner = vec![];
        inner.resize_with(size, UnsafeCell::default);

        Self { inner: inner.into() }
    }

    pub fn insert(&self, key: u64, value: T) {
        let hash = hash64(&bytemuck::bytes_of(&value));
        let entry = TableEntry { key, hash, value };
        unsafe { *self.inner[key as usize % self.inner.len()].get() = entry; }
    }

    pub fn get(&self, key: u64) -> Option<T> {
        let entry = unsafe { (*self.inner[key as usize % self.inner.len()].get()).clone() };
        let value = entry.value;

        (entry.key == key && entry.hash == hash64(&bytemuck::bytes_of(&value))).then_some(value)
    }
}
