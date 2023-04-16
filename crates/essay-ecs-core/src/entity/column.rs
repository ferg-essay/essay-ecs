use core::fmt;
use std::mem::{self, ManuallyDrop};
use std::ptr::NonNull;
use std::{cmp};
use std::alloc::Layout;

use super::meta::{ColumnId, ColumnType, StoreMeta};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct RowId(u32);

pub(crate) struct Column {
    meta: ColumnType,

    inc: usize,
    pad_size: usize,

    data: NonNull<u8>,
    
    len: usize,
    capacity: usize,

    //marker: PhantomData<&'c u8>,
}

impl RowId {
    pub const INVALID: RowId = RowId(u32::MAX);

    pub fn new(index: usize) -> RowId {
        RowId(index as u32)
    }
        
    #[inline]
    pub fn index(&self) -> usize {
        self.0 as usize
    }
}

impl Column {
    pub(crate) fn new<T:'static>(metas: &mut StoreMeta) -> Self {
        let id = metas.add_column::<T>();
        let meta = metas.column(id);

        let pad_size = meta.layout_padded().size();

        let inc: usize = if mem::size_of::<T>() < 8 {
            8
        } else if mem::size_of::<T>() < 64 {
            4
        } else {
            1
        };
            
        // zero-length items are pre-allocated
        let length = if pad_size == 0 { 1 } else { 0 };
        let capacity = length;

        let data = dangling_data(meta.layout_padded().align());
        
        Self {
            meta: meta.clone(),

            pad_size: pad_size,
            inc: inc,

            data: data,

            len: length,
            capacity: capacity,

            // marker: Default::default(),
        }
    }
    
    pub fn id(&self) -> ColumnId {
        self.meta.id()
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    pub(crate) unsafe fn get<T>(&self, row: RowId) -> Option<&T> {
        let index = row.index();
        
        if index < self.len {
            let offset = self.offset(index);

            Some(&*self.data.as_ptr().add(offset).cast::<T>())
        } else {
            None
        }
    }
    
    pub(crate) unsafe fn get_mut<T>(&self, row: RowId) -> Option<&mut T> {
        let index = row.index();

        if index < self.len {
            let offset = self.offset(index);

            Some(&mut *self.data.as_ptr().add(offset).cast::<T>())
        } else {
            None
        }
    }

    pub(crate) unsafe fn push<T>(&mut self, value: T) -> RowId {
        self.reserve(1);

        let index = self.len;

        self.write(index, value);
        
        self.len += 1;

        RowId::new(index)
    }

    unsafe fn write<T>(&mut self, index: usize, value: T) {
        assert!(index < self.capacity);

        let mut value = ManuallyDrop::new(value);
        let source: NonNull<u8> = NonNull::from(&mut *value).cast();

        let src = source.as_ptr();

        let offset = self.offset(index);
        let dst = self.data.as_ptr().add(offset);

        let count = mem::size_of::<T>();

        std::ptr::copy_nonoverlapping::<u8>(src, dst, count);
    }
    
    #[inline]
    fn offset(&self, index: usize) -> usize {
        self.pad_size * index
    }

    pub(crate) fn reserve(&mut self, len: usize) {
        let avail = self.capacity - self.len;

        if avail < len {
            let delta = cmp::max(self.inc, len - avail);

            self.extend(self.len + delta);
        }
    }

    pub(crate) fn reserve_exact(&mut self, len: usize) {
        let avail = self.capacity - self.len;

        if len < avail {
            let delta = len - avail;

            self.extend(self.len + delta);
        }
    }

    fn extend(&mut self, new_capacity: usize) {
        assert!(self.pad_size > 0, "zero sized column items can't be pushed");
        assert!(self.capacity < new_capacity);

        let layout = self.array_layout(new_capacity);

        let data = if self.capacity == 0 {
            unsafe {
                std::alloc::alloc(layout)
            }
        } else {
            unsafe {
                std::alloc::realloc(
                    self.data.as_ptr(),
                    layout,
                    layout.size(),
                 )
            }
        };

        self.data = NonNull::new(data).unwrap();
        self.capacity = new_capacity;
    }
    
    fn array_layout(&mut self, n: usize) -> Layout {
        unsafe {
            let size = n * self.meta.size_padded();
            Layout::from_size_align_unchecked(size, self.meta.layout_padded().align())
        }
    }
}

impl fmt::Debug for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Column")
         .field("id", &self.id())
         .field("name", &self.meta.name())
         .field("pad_size", &self.pad_size)
         .field("len", &self.len())
         .finish()
    }
}

fn dangling_data(align: usize) -> NonNull<u8> {
    if align > 0 {
        assert!(align.is_power_of_two());

        unsafe { NonNull::new_unchecked(align as *mut u8) }
    } else {
        unsafe { NonNull::new_unchecked(8 as *mut u8) }
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::{meta::StoreMeta, column::RowId};

    use super::Column;

    #[test]
    fn col_null() {
        let mut metas = StoreMeta::new();
        let col = Column::new::<()>(&mut metas);

        assert_eq!(col.capacity(), 1);
        assert_eq!(col.len(), 1);
        
        //assert_eq!(col.push(()), 0);
        unsafe {
            assert_eq!(col.get::<()>(RowId::new(0)), Some(&()));
            assert_eq!(col.get::<()>(RowId::new(1)), None);
        }
    }

    #[test]
    fn col_u8() {
        let mut metas = StoreMeta::new();
        let mut col = Column::new::<u8>(&mut metas);

        assert_eq!(col.capacity(), 0);
        assert_eq!(col.len(), 0);
        
        unsafe {
            assert_eq!(col.get::<u8>(RowId::new(0)), None);

            assert_eq!(col.push::<u8>(1), RowId::new(0));
        }

        assert_eq!(col.capacity(), 8);
        assert_eq!(col.len(), 1);

        unsafe {
            assert_eq!(col.get::<u8>(RowId::new(0)), Some(&1));
            assert_eq!(col.get::<u8>(RowId::new(1)), None);

            assert_eq!(col.push::<u8>(2), RowId::new(1));
        }

        assert_eq!(col.capacity(), 8);
        assert_eq!(col.len(), 2);

        unsafe {
            assert_eq!(col.get::<u8>(RowId::new(0)), Some(&1));
            assert_eq!(col.get::<u8>(RowId::new(1)), Some(&2));
            assert_eq!(col.get::<u8>(RowId::new(2)), None);
        }
    }

    #[test]
    fn col_u16() {
        let mut metas = StoreMeta::new();
        let mut col = Column::new::<TestA>(&mut metas);

        assert_eq!(col.capacity(), 0);
        assert_eq!(col.len(), 0);
        
        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), None);

            assert_eq!(col.push::<TestA>(TestA(1)), RowId::new(0));
        }

        assert_eq!(col.capacity(), 8);
        assert_eq!(col.len(), 1);

        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), Some(&TestA(1)));
            assert_eq!(col.get::<TestA>(RowId::new(1)), None);

            assert_eq!(col.push::<TestA>(TestA(1002)), RowId::new(1));   
        }

        assert_eq!(col.capacity(), 8);
        assert_eq!(col.len(), 2);

        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), Some(&TestA(1)));
            assert_eq!(col.get::<TestA>(RowId::new(1)), Some(&TestA(1002)));
            assert_eq!(col.get::<TestA>(RowId::new(2)), None);
        }
    }

    #[derive(Debug, PartialEq)]
    struct TestA(u16);
}
