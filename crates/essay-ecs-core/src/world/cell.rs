use std::fmt::Pointer;
use std::mem::{self, ManuallyDrop};
use std::ptr::NonNull;
use std::{marker::PhantomData, cmp};
use std::alloc::Layout;

pub(crate) struct Ptr {
    data: NonNull<u8>,
}

impl Ptr {
    pub(crate) fn new<T>(value: T) -> Self {
        let layout = Layout::new::<T>();
        let data = unsafe { std::alloc::alloc(layout) };
        let data = NonNull::new(data).unwrap();

        let mut cell = Self {
            data: data,

            // marker: Default::default(),
        };

        unsafe {
            cell.write(value);
        }

        cell
    }
    
    pub unsafe fn deref<T>(&self) -> &T {
        &*self.data.as_ptr().cast::<T>()
    }
    
    pub unsafe fn deref_mut<T>(&self) -> &mut T {
        &mut *self.data.as_ptr().cast::<T>()
    }

    unsafe fn write<T>(&mut self, value: T) {
        let mut value = ManuallyDrop::new(value);
        let source: NonNull<u8> = NonNull::from(&mut *value).cast();

        let src = source.as_ptr();
        let dst = self.data.as_ptr();

        let count = mem::size_of::<T>();

        std::ptr::copy_nonoverlapping::<u8>(src, dst, count);
    }
}

#[derive(Copy, Clone)]
pub struct PtrOwn<'a>(NonNull<u8>, PhantomData<&'a mut u8>);

impl<'a> PtrOwn<'a> {
    #[inline]
    pub fn new(data: NonNull<u8>) -> Self {
        Self(data, PhantomData)
    }

    pub fn spawn<T, F, R>(value: T, fun: F) -> R
        where F: FnOnce(PtrOwn<'a>) -> R
    {
        let mut value = ManuallyDrop::new(value);
        
        fun(Self::new(NonNull::from(&mut *value).cast()))
    }

    pub unsafe fn make_into<T>(value: T, storage: &mut NonNull<u8>) -> Self {
        let len = mem::size_of::<T>();

        let mut value = ManuallyDrop::new(value);
        let source: NonNull<u8> = NonNull::from(&mut *value).cast();

        std::ptr::copy_nonoverlapping::<u8>(
            source.as_ptr(), 
            storage.as_ptr(),
            len
        );
    
        PtrOwn::new(*storage)
    }

    pub unsafe fn write<T>(&mut self, value: T) {
        let len = mem::size_of::<T>();

        let mut value = ManuallyDrop::new(value);
        let source: NonNull<u8> = NonNull::from(&mut *value).cast();

        std::ptr::copy_nonoverlapping::<u8>(
            source.as_ptr(), 
            self.0.as_ptr(),
            len
        );
    }

    #[inline]
    pub unsafe fn deref<T>(self) -> &'a T {
        &*self.as_ptr().cast::<T>() // .debug_ensure_aligned()
    }

    #[inline]
    pub unsafe fn deref_mut<T>(self) -> &'a mut T {
        &mut *self.as_ptr().cast::<T>() // .debug_ensure_aligned()
    }

    #[inline]
    pub fn as_ptr(self) -> *mut u8 {
        self.0.as_ptr()
    }
}

impl<'a> From<PtrOwn<'a>> for NonNull<u8> {
    fn from(value: PtrOwn<'a>) -> Self {
        value.0
    }
}

impl Pointer for PtrOwn<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Pointer::fmt(&self.0, f)
    }
}

pub struct PtrCell<'t, T> {
    data: Vec<u8>,
    ptr: PtrOwn<'t>,
    marker: PhantomData<T>,
}

// TODO: alignment, drop, columns, non-vec backing
impl<'t, T> PtrCell<'t, T> {
    pub fn new(value: T) -> Self {
        let len = mem::size_of::<T>();

        let mut data = Vec::<u8>::new();
        data.resize(len, 0); // TODO: ignoring alignment

        let mut storage = unsafe { NonNull::new_unchecked(data.as_mut_ptr()) };

        let ptr = unsafe { PtrOwn::make_into(value, &mut storage) };

        Self {
            data: data,
            ptr: ptr,
            marker: PhantomData,
        }
    }

    pub unsafe fn deref(&self) -> &T {
        self.ptr.deref()
    }

    pub unsafe fn deref_mut(&self) -> &mut T {
        self.ptr.deref_mut()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ptr_null() {
        let ptr = Ptr::new::<()>(());
        
        unsafe { assert_eq!(ptr.deref::<()>(), &()); }
    }

    #[test]
    fn ptr_u8() {
        let mut ptr = Ptr::new::<u8>(1);

        unsafe { assert_eq!(ptr.deref::<u8>(), &1); }
        unsafe { assert_eq!(ptr.deref_mut::<u8>(), &1); }

        unsafe { *ptr.deref_mut::<u8>() = 3; }

        unsafe { assert_eq!(ptr.deref::<u8>(), &3); }
        unsafe { assert_eq!(ptr.deref_mut::<u8>(), &3); }
    }
    use std::{mem, ptr::NonNull};

    use crate::world::cell::Ptr;

    use super::{PtrOwn};

    #[test]
    fn test_ptr_own_to_and_from() {
        let test = Test32(1);
        let size = mem::size_of::<Test32>();
        let mut vec = Vec::<u8>::new();
        vec.resize(size, 0);

        let mut data = unsafe { NonNull::new_unchecked(vec.as_mut_ptr()) };

        let ptr =  unsafe { PtrOwn::make_into(test, &mut data) };

        let test2 = unsafe { ptr.deref::<Test32>() };

        assert_eq!(format!("{:?}", test2), "Test32(1)");
    }

    #[derive(Debug)]
    struct Test(String);

    #[derive(Debug)]
    struct Test32(u32);

    #[derive(Debug)]
    struct TestValue {
        value: String
    }
}
