use crate::{store::{CommandQueue, Commands}, schedule::{SystemMeta, UnsafeWorld}, Store};

use super::Param;

impl Param for Commands<'_, '_> {
    type Arg<'w, 's> = Commands<'w, 's>;
    type State = CommandQueue;

    fn init(_meta: &mut SystemMeta, _world: &mut Store) -> Self::State {
        CommandQueue::default()
    }

    fn arg<'w,'s>(
        world: &'w UnsafeWorld,
        queue: &'s mut Self::State, 
    ) -> Self::Arg<'w, 's> {
        unsafe { Commands::new(world.as_mut(), queue) }
    }

    fn flush(world: &mut Store, queue: &mut Self::State) {
        queue.flush(world);
    }
}
