use std::any::type_name;

use crate::{store::Store, schedule::{SystemMeta, UnsafeWorld}};

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct SystemId(pub(crate) usize);

pub trait System: Send + Sync + 'static {
    type Out;

    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }

    fn init(&mut self, meta: &mut SystemMeta, world: &mut Store);

    unsafe fn run_unsafe(&mut self, world: &UnsafeWorld) -> Self::Out;

    fn run(&mut self, world: &mut UnsafeWorld) -> Self::Out {
        unsafe { self.run_unsafe(&world) }
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
