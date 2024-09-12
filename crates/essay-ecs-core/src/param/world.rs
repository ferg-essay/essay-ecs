use crate::{
    error::Result,
    schedule::{SystemMeta, UnsafeStore}, 
    Store
};

use super::Param;

impl Param for &Store {
    type Arg<'w, 's> = &'w Store;
    type Local = ();

    fn arg<'w, 's>(
        world: &'w UnsafeStore,
        _state: &'s mut Self::Local,
    ) -> Result<Self::Arg<'w, 's>> {
        Ok(world)
    }

    fn init(meta: &mut SystemMeta, _world: &mut Store) -> Result<Self::Local> {
        meta.set_exclusive();

        Ok(())
    }
}
