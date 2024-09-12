use std::any::type_name;

use crate::{
    error::Result,
    store::Store, 
    schedule::{SystemMeta, UnsafeStore}
};

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct SystemId(pub(crate) usize);

pub trait System: Send + Sync + 'static {
    type Out;

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn init(&mut self, meta: &mut SystemMeta, store: &mut Store) -> Result<()>;

    unsafe fn run_unsafe(&mut self, store: &UnsafeStore) -> Result<Self::Out>;

    fn run(&mut self, store: &mut UnsafeStore) -> Result<Self::Out> {
        unsafe { self.run_unsafe(&store) }
    }

    fn flush(&mut self, world: &mut Store);
}

pub trait IntoSystem<Out, M>: Sized {
    type System: System<Out = Out>;

    fn into_system(this: Self) -> Self::System;
}

impl SystemId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl<S, Out> IntoSystem<Out, ()> for S
where
    S: System<Out = Out>
{
    type System = S;

    fn into_system(this: Self) -> Self::System {
        this
    }
}
