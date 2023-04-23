
//
// Local param
//

use std::ops::{Deref, DerefMut};

use crate::{world::FromWorld, schedule::{SystemMeta, UnsafeWorld}, World};

use super::Param;

pub struct Local<'s, T:FromWorld>(pub(crate) &'s mut T);

impl<'s, T:FromWorld> Deref for Local<'s, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'s, T:FromWorld> DerefMut for Local<'s, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
    }
}

impl<'a, T:FromWorld+Send+Sync+'static> Param for Local<'a, T> {
    type State = T;
    type Arg<'w, 's> = Local<'s, T>;

    fn init(_meta: &mut SystemMeta, world: &mut World) -> Self::State {
        // let exl = std::sync::Exclusive::new(T::default());
        T::init(world)
    }

    fn arg<'w, 's>(
        _world: &'w UnsafeWorld, 
        state: &'s mut Self::State, 
    ) -> Self::Arg<'w, 's> {
        Local(state)
    }

    fn flush(_world: &mut World, _state: &mut Self::State) {
    }
}


#[cfg(test)]
mod tests {
    use crate::{world::{World}, schedule::Schedule, param::ResMut};

    use super::Local;

    #[test]
    fn test_local() {
        let mut world = World::new();
        world.insert_resource::<String>("none".to_string());

        let mut schedule = Schedule::new();
        schedule.add_system(local_system);

        schedule.tick(&mut world).unwrap();
        assert_eq!(world.resource::<String>(), "local(1)");

        schedule.tick(&mut world).unwrap();
        assert_eq!(world.resource::<String>(), "local(2)");
    }

    fn local_system(mut local: Local<usize>, mut value: ResMut<String>) {
        *local = *(local.0) + 1;
        *value = format!("local({})", local.0);
    }

}