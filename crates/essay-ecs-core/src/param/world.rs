use crate::{schedule::SystemMeta, World};

use super::Param;

impl Param for &World {
    type Arg<'w, 's> = &'w World;
    type State = ();

    fn arg<'w, 's>(
        world: &'w World,
        _state: &'s mut Self::State,
    ) -> Self::Arg<'w, 's> {
        world
    }

    fn init(meta: &mut SystemMeta, _world: &mut World) -> Self::State {
        meta.set_exclusive();

        ()
    }
}
