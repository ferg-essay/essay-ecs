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

    #[derive(Debug)]
    struct Test(String);

    #[derive(Debug)]
    struct Test32(u32);

    #[derive(Debug)]
    struct TestValue {
        value: String
    }
}
