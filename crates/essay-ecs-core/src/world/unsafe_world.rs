use std::{marker::PhantomData, cell::UnsafeCell};

use crate::entity::{View, ViewIterator};

use super::World;

pub struct UnsafeWorld<'w>(*mut World, PhantomData<(&'w World, &'w UnsafeCell<World>)>);

impl<'w> UnsafeWorld<'w> {
    pub(crate) fn new_readonly(world: &'w World) -> Self {
        Self(world as *const World as *mut World, PhantomData)
    }

    pub(crate) fn new_mutable(world: &'w mut World) -> Self {
        Self(world as *mut World, PhantomData)
    }

    pub unsafe fn world_mut(&self) -> &'w mut World {
        unsafe { &mut *self.0 }
    }

    pub unsafe fn world(&self) -> &'w World {
        unsafe { &*self.0 }
    }

    /*
    pub(crate) unsafe fn view<V:View>(&self) -> ViewIterator<V> {
        //self.world_mut().table.iter_view::<V>()
    }
    */
}