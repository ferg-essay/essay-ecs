use crate::{schedule::{SystemMeta, UnsafeStore}, Store};

use super::Param;

impl Param for &Store {
    type Arg<'w, 's> = &'w Store;
    type State = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::State,
    ) -> Self::Arg<'w, 's> {
        world
    }

    fn init(meta: &mut SystemMeta, _world: &mut Store) -> Self::State {
        meta.set_exclusive();

        ()
    }
}
