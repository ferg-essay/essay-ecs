use std::{marker::PhantomData, cell::UnsafeCell};

use crate::entity::{View, ViewIterator};

use super::World;
/*
pub struct UnsafeWorld(World);

impl UnsafeWorld {
    pub(crate) fn new(world: World) -> Self {
        UnsafeWorld(world)
    }

    pub fn take(self) -> World {
        self.0
    }
}

unsafe impl Send for UnsafeWorld {
}
*/