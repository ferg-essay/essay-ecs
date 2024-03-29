use core::fmt;
use std::mem::{self, ManuallyDrop};
use std::ptr::NonNull;
use std::cmp;
use std::alloc::Layout;

use super::meta::{ColumnId, ColumnType, StoreMeta};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct RowId(u32, u32);

pub(crate) struct Column {
    meta: ColumnType,

    inc: usize,
    pad_size: usize,

    data: NonNull<u8>,
    row_gen: Vec<u32>,
    
    len: usize,
    capacity: usize,

    free_list: Vec<RowId>,

    //drop: Option<Box<dyn Fn(&mut Column, usize)>>,
    drop: Option<Box<dyn Fn(&mut Column, usize) -> bool>>,
}

impl RowId {
    pub const FREE_MASK: u32 = 0x8000_0000;
    pub const UNSET: RowId = RowId(u32::MAX, RowId::FREE_MASK);

    pub fn new(index: usize) -> RowId {
        RowId(index as u32, 0)
    }
        
    #[inline]
    pub fn index(&self) -> usize {
        self.0 as usize
    }
        
    #[inline]
    pub fn gen(&self) -> u32 {
        self.1
    }

    #[inline]
    pub(crate) fn is_alloc(&self) -> bool {
        self.1 & Self::FREE_MASK == 0
    }

    pub(crate) fn next_free(&self) -> RowId {
        assert!(self.1 & Self::FREE_MASK == 0);

        RowId(self.0, (self.1 + 1) | Self::FREE_MASK)
    }

    pub(crate) fn _next(&self) -> RowId {
        assert!(self.1 & Self::FREE_MASK == 0);

        RowId(self.0, (self.1 + 1) & !Self::FREE_MASK)
    }

    pub(crate) fn allocate(&self) -> RowId {
        assert!(self.1 & Self::FREE_MASK != 0);

        RowId(self.0, self.1 & !Self::FREE_MASK)
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
        let mut row_gen = Vec::new();
        for _ in 0..length {
            row_gen.push(0);
        }
        let capacity = length;

        let data = dangling_data(meta.layout_padded().align());
        
        let drop = Box::new(|c: &mut Column, i: usize| c.drop_index::<T>(i));

        Self {
            meta: meta.clone(),

            pad_size: pad_size,
            inc: inc,

            data: data,
            row_gen,

            len: length,
            capacity: capacity,

            free_list: Default::default(),

            drop: Some(drop),
        }
    }
    
    pub fn id(&self) -> ColumnId {
        self.meta.id()
    }

    #[inline]
    pub fn _capacity(&self) -> usize {
        self.capacity
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn _is_empty(&self) -> bool {
        self.len == 0
    }
    
    pub(crate) unsafe fn get<T>(&self, row: RowId) -> Option<&T> {
        let index = row.index();
        
        if index < self.len && self.row_gen[index] == row.gen() {
            let offset = self.offset(index);

            Some(&*self.data.as_ptr().add(offset).cast::<T>())
        } else {
            None
        }
    }
    
    pub(crate) unsafe fn get_mut<T>(&self, row: RowId) -> Option<&mut T> {
        let index = row.index();

        if index < self.len && self.row_gen[index] == row.gen() {
            let offset = self.offset(index);

            Some(&mut *self.data.as_ptr().add(offset).cast::<T>())
        } else {
            None
        }
    }

    pub(crate) unsafe fn push<T>(&mut self, value: T) -> RowId {
        if let Some(id) = self.free_list.pop() {
            assert_eq!(id.gen(), self.row_gen[id.index()]);
            let id = id.allocate();

            self.row_gen[id.index() as usize] = id.gen();

            self.write(id.index(), value);

            id
        } else {
            self.reserve(1);

            let index = self.len;

            self.write(index, value);
            self.row_gen.push(0);
        
            self.len += 1;

            RowId::new(index)
        }
    }

    pub(crate) fn remove(&mut self, row: RowId) {
        assert!(row.is_alloc());

        let drop = self.drop.take().unwrap();

        if drop(self, row.index()) {
            self.free_list.push(row.next_free());
        }

        self.drop.replace(drop);
    }

    #[cfg(test)]
    pub(crate) unsafe fn insert<T>(&mut self, row: RowId, value: T) -> Option<RowId> {
        let index = row.index();

        if index < self.len() && self.row_gen[index] == row.gen() {
            self.row_gen[index] = (self.row_gen[index] + 1) & ! RowId::FREE_MASK;

            unsafe {
                let offset = self.offset(index);

                self.data.as_ptr()
                    .add(offset)
                    .cast::<T>()
                    .drop_in_place();

                self.write(index, value);
            }

            Some(RowId(row.0, self.row_gen[index]))
        } else {
            None
        }
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
 
    fn drop_index<T>(&mut self, index: usize) -> bool {
        if index < self.len() && self.row_gen[index] & RowId::FREE_MASK == 0 {
            self.row_gen[index] = self.row_gen[index] + 1 | RowId::FREE_MASK;

            unsafe {
                let offset = self.offset(index);

                self.data.as_ptr()
                    .add(offset)
                    .cast::<T>()
                    .drop_in_place();
            }

            true
        } else {
            false
        }
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

    fn extend(&mut self, new_capacity: usize) {
        assert!(self.pad_size > 0, "zero sized column items can't be pushed");
        assert!(self.capacity < new_capacity);

        self.row_gen.reserve_exact(new_capacity - self.capacity);

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

impl Drop for Column {
    fn drop(&mut self) {
        let len = self.len();
        let drop = self.drop.take().unwrap();

        for i in 0..len {
            drop(self, i);
        }
        self.drop.replace(drop);
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
    use std::{rc::Rc, cell::RefCell};

    use crate::entity::{meta::StoreMeta, column::RowId};

    use super::Column;

    #[test]
    fn col_null() {
        let mut metas = StoreMeta::new();
        let col = Column::new::<()>(&mut metas);

        assert_eq!(col._capacity(), 1);
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

        assert_eq!(col._capacity(), 0);
        assert_eq!(col.len(), 0);
        
        unsafe {
            assert_eq!(col.get::<u8>(RowId::new(0)), None);

            assert_eq!(col.push::<u8>(1), RowId::new(0));
        }

        assert_eq!(col._capacity(), 8);
        assert_eq!(col.len(), 1);

        unsafe {
            assert_eq!(col.get::<u8>(RowId::new(0)), Some(&1));
            assert_eq!(col.get::<u8>(RowId::new(1)), None);

            assert_eq!(col.push::<u8>(2), RowId::new(1));
        }

        assert_eq!(col._capacity(), 8);
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

        assert_eq!(col._capacity(), 0);
        assert_eq!(col.len(), 0);
        
        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), None);

            assert_eq!(col.push::<TestA>(TestA(1)), RowId::new(0));
        }

        assert_eq!(col._capacity(), 8);
        assert_eq!(col.len(), 1);

        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), Some(&TestA(1)));
            assert_eq!(col.get::<TestA>(RowId::new(1)), None);

            assert_eq!(col.push::<TestA>(TestA(1002)), RowId::new(1));   
        }

        assert_eq!(col._capacity(), 8);
        assert_eq!(col.len(), 2);

        unsafe {
            assert_eq!(col.get::<TestA>(RowId::new(0)), Some(&TestA(1)));
            assert_eq!(col.get::<TestA>(RowId::new(1)), Some(&TestA(1002)));
            assert_eq!(col.get::<TestA>(RowId::new(2)), None);
        }
    }

    #[test]
    fn remove_push() {
        let mut metas = StoreMeta::new();

        let mut col = Column::new::<TestA>(&mut metas);

        unsafe {
            let id_0 = col.push::<TestA>(TestA(0));
            assert_eq!(id_0, RowId::new(0));

            let id_1 = col.push::<TestA>(TestA(1));
            assert_eq!(id_1, RowId::new(1));

            let id_2 = col.push::<TestA>(TestA(2));
            assert_eq!(id_2, RowId::new(2));

            let id_3 = col.push::<TestA>(TestA(3));
            assert_eq!(id_3, RowId::new(3));

            col.remove(id_0);
            col.remove(id_1);

            assert_eq!(col.get::<TestA>(id_0), None);
            assert_eq!(col.get::<TestA>(id_1), None);
            assert_eq!(col.get::<TestA>(id_2), Some(&TestA(2)));
            assert_eq!(col.get::<TestA>(id_3), Some(&TestA(3)));

            let id_4 = col.push::<TestA>(TestA(4));
            assert_eq!(id_4, RowId(1, 1));

            let id_5 = col.push::<TestA>(TestA(5));
            assert_eq!(id_5, RowId(0, 1));

            let id_6 = col.push::<TestA>(TestA(6));
            assert_eq!(id_6, RowId(4, 0));

            assert_eq!(col.get::<TestA>(id_0), None);
            assert_eq!(col.get::<TestA>(id_1), None);
            assert_eq!(col.get::<TestA>(id_2), Some(&TestA(2)));
            assert_eq!(col.get::<TestA>(id_3), Some(&TestA(3)));
            assert_eq!(col.get::<TestA>(id_4), Some(&TestA(4)));
            assert_eq!(col.get::<TestA>(id_5), Some(&TestA(5)));
            assert_eq!(col.get::<TestA>(id_6), Some(&TestA(6)));
        }
    }

    #[test]
    fn column_drop() {
        let mut metas = StoreMeta::new();

        let value = Rc::new(RefCell::new(Vec::<String>::new()));
        
        {
            let mut col = Column::new::<TestDrop>(&mut metas);

            unsafe {
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 10)), RowId::new(0));
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 20)), RowId::new(1));
            }

            assert_eq!(take(&value), "");
        }
        
        assert_eq!(take(&value), "drop[10], drop[20]");
    }

    #[test]
    fn remove_drop() {
        let mut metas = StoreMeta::new();

        let value = Rc::new(RefCell::new(Vec::<String>::new()));
        
        {
            let mut col = Column::new::<TestDrop>(&mut metas);

            unsafe {
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 10)), RowId::new(0));
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 20)), RowId::new(1));
            }

            assert_eq!(take(&value), "");

            col.remove(RowId::new(0));
        
            assert_eq!(take(&value), "drop[10]");
        }

        assert_eq!(take(&value), "drop[20]");
    }

    #[test]
    fn insert_drop() {
        let mut metas = StoreMeta::new();

        let value = Rc::new(RefCell::new(Vec::<String>::new()));
        
        {
            let mut col = Column::new::<TestDrop>(&mut metas);

            unsafe {
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 10)), RowId::new(0));
                assert_eq!(col.push::<TestDrop>(TestDrop(value.clone(), 20)), RowId::new(1));
            }

            assert_eq!(take(&value), "");

            unsafe {
                assert_eq!(col.insert::<TestDrop>(RowId::new(0), TestDrop(value.clone(), 110)),
                    Some(RowId(0, 1)));
            }
        
            assert_eq!(take(&value), "drop[10]");
        }

        assert_eq!(take(&value), "drop[110], drop[20]");
    }

    fn take(value: &Rc<RefCell<Vec<String>>>) -> String {
        let values : Vec<String> = value.borrow_mut().drain(..).collect();

        values.join(", ")
    }

    #[derive(Debug, PartialEq)]
    struct TestA(u16);

    #[derive(Debug, PartialEq)]
    struct TestDrop(Rc<RefCell<Vec<String>>>, usize);

    impl Drop for TestDrop {
        fn drop(&mut self) {
            self.0.borrow_mut().push(format!("drop[{:?}]", self.1));
        }
    }
}
