use std::cell::UnsafeCell;

pub struct SyncUnsafeCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for SyncUnsafeCell<T> {}
unsafe impl<T: ?Sized + Send> Send for SyncUnsafeCell<T> {}

impl<T> SyncUnsafeCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    pub(crate) fn get(&self) -> &T {
        unsafe { self.value.get().as_ref().unwrap() }
    }

    pub(crate) fn as_mut(&self) -> &mut T {
        unsafe { &mut *self.value.get() }
    }

    pub(crate) fn get_ptr(&self) -> *mut T {
        self.value.get()
    }
}
