use std::{cell::UnsafeCell, ops::{Deref, DerefMut}};

use crate::World;

pub struct UnsafeSyncCell<T: ?Sized> {
    value: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Sync> Sync for UnsafeSyncCell<T> {}
unsafe impl<T: ?Sized + Send> Send for UnsafeSyncCell<T> {}

impl<T> UnsafeSyncCell<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    pub(crate) fn get_ref(&self) -> &T {
        unsafe { self.value.get().as_ref().unwrap() }
    }

    pub(crate) unsafe fn as_mut(&self) -> &mut T {
        &mut *self.value.get()
    }

    pub(crate) fn _get(&self) -> *mut T {
        self.value.get()
    }
}

pub struct UnsafeSendCell<T>(UnsafeCell<T>);

impl<T> UnsafeSendCell<T> {
    pub fn new(value: T) -> Self {
        UnsafeSendCell(UnsafeCell::new(value))
    }

    pub(crate) unsafe fn get_ref(&self) -> &T {
        self.0.get().as_ref().unwrap()
    }

    pub(crate) unsafe fn as_mut(&self) -> &mut T {
        &mut *self.0.get()
    }

    pub(crate) fn _take(self) -> T {
        self.0.into_inner()
    }
}

unsafe impl<T> Send for UnsafeSendCell<T> {}
unsafe impl<T> Sync for UnsafeSendCell<T> {}

pub struct UnsafeWorld(UnsafeCell<World>);

impl UnsafeWorld {
    pub fn new(world: World) -> Self {
        UnsafeWorld(UnsafeCell::new(world))
    }

    pub(crate) unsafe fn get_ref(&self) -> &World {
        unsafe { self.0.get().as_ref().unwrap() }
    }

    pub fn get_mut(&mut self) -> &mut World {
        self.0.get_mut()
    }

    pub(crate) unsafe fn as_mut(&self) -> &mut World {
        &mut *self.0.get()
    }

    pub(crate) fn take(self) -> World {
        self.0.into_inner()
    }
}

impl Deref for UnsafeWorld {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        unsafe { self.get_ref() }
    }
}

impl DerefMut for UnsafeWorld {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}
