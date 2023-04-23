use std::{ops::{Deref, DerefMut}};

use crate::{schedule::{SystemMeta, UnsafeWorld}, World};

use super::Param;

#[derive(Debug)]
pub struct Res<'a, T> {
    value: &'a T,
}

impl<'a, T:'static> Res<'a, T> {
    pub fn get(&self) -> &T {
        self.value
    }
}

impl<T:Send+'static> Param for Res<'_, T> {
    type Arg<'w, 's> = Res<'w, T>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeWorld,
        _state: &'s mut Self::State,
    ) -> Res<'w, T> {
        Res {
            value: world.get_resource::<T>().unwrap(),
        }
    }

    fn init(meta: &mut SystemMeta, world: &mut World) -> Self::State {
        meta.insert_resource(world.get_resource_id::<T>());
    }
}

impl<T:'static> Deref for Res<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct ResMut<'a, T> {
    value: &'a mut T,
}

impl<T:'static> ResMut<'_, T> {
    pub fn get(&self) -> &T {
        self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.value
    }
}

impl<T:'static> Deref for ResMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<'a, T:'static> DerefMut for ResMut<'_, T> {
    // type Target = T;

    fn deref_mut(&mut self) -> &mut Self::Target {
        self.get_mut()
    }
}

impl<T:Send+'static> Param for ResMut<'_, T> {
    type Arg<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init(meta: &mut SystemMeta, world: &mut World) -> Self::State {
        meta.insert_resource_mut(world.get_resource_id::<T>());
    }

    fn arg<'w, 's>(
        world: &'w UnsafeWorld,
        _state: &'s mut Self::State,
    ) -> ResMut<'w, T> {
        ResMut {
            value: unsafe { world.as_mut().get_resource_mut().unwrap() }
        }
    }
}

