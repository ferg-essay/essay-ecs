use std::{ops::{Deref, DerefMut}};

use crate::{prelude::{Param}, schedule::SystemMeta};

use super::World;


#[derive(Debug)]
pub struct Res<'a, T> {
    value: &'a T,
}

impl<'a, T:'static> Res<'a, T> {
    pub fn get(&self) -> &T {
        self.value
    }
}

impl<'a, T:'static> Param for Res<'_, T> {
    type Arg<'w, 's> = Res<'w, T>;
    type State = ();

    fn arg<'w, 's>(
        world: &'w World,
        state: &'s mut Self::State,
    ) -> Res<'w, T> {
        Res {
            value: world.get_resource::<T>().unwrap(),
        }
    }

    fn init(world: &mut World, meta: &mut SystemMeta) -> Self::State {
        ()
    }

    fn flush(world: &mut World, state: &mut Self::State) {
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

impl<T:'static> Param for ResMut<'_, T> {
    type Arg<'w, 's> = ResMut<'w, T>;
    type State = ();

    fn init(world: &mut World, _meta: &mut SystemMeta) -> Self::State {
        ()
    }

    fn arg<'w, 's>(
        world: &'w World,
        _state: &'s mut Self::State,
    ) -> ResMut<'w, T> {
        ResMut {
            value: world.get_resource_mut().unwrap()
        }
    }

    fn flush(world: &mut World, state: &mut Self::State) {
    }
}

impl Param for &World {
    type Arg<'w, 's> = &'w World;
    type State = ();

    fn arg<'w, 's>(
        world: &'w World,
        state: &'s mut Self::State,
    ) -> Self::Arg<'w, 's> {
        world
    }

    fn init(world: &mut World, meta: &mut SystemMeta) -> Self::State {
    }

    fn flush(world: &mut World, state: &mut Self::State) {
    }
}

