use crate::{
    error::Result,
    store::{CommandQueue, Commands}, 
    schedule::{SystemMeta, UnsafeStore}, 
    Store
};

use super::Param;

impl Param for Commands<'_, '_> {
    type Arg<'w, 's> = Commands<'w, 's>;
    type State = CommandQueue;

    fn init(_meta: &mut SystemMeta, _world: &mut Store) -> Result<Self::State> {
        Ok(CommandQueue::default())
    }

    fn arg<'w,'s>(
        world: &'w UnsafeStore,
        queue: &'s mut Self::State, 
    ) -> Result<Self::Arg<'w, 's>> {
        unsafe { Ok(Commands::new(world.as_mut(), queue)) }
    }

    fn flush(world: &mut Store, queue: &mut Self::State) {
        queue.flush(world);
    }
}
