use std::ops::{Deref, DerefMut};

use crate::{world::{World, FromWorld}, schedule::SystemMeta};

//
// Param
//
 
pub trait Param {
    type Arg<'w, 's>: Param<State=Self::State>;
    type State: Sync + Send + 'static;

    fn init(meta: &mut SystemMeta, world: &mut World) -> Self::State;

    fn arg<'w, 's>(
        world: &'w World,
        state: &'s mut Self::State, 
    ) -> Self::Arg<'w, 's>;

    fn flush(_world: &mut World, _state: &mut Self::State) {
    }
}

pub type Arg<'w, 's, P> = <P as Param>::Arg<'w, 's>;

//
// Param composed of tuples
//

macro_rules! impl_param_tuple {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($param: Param,)*> Param for ($($param,)*)
        {
            type Arg<'w, 's> = ($($param::Arg<'w, 's>,)*);
            type State = ($(<$param as Param>::State,)*);

            fn init(
                meta: &mut SystemMeta,
                world: &mut World, 
            ) -> Self::State {
                ($($param::init(meta, world),)*)
            }

            fn arg<'w, 's>(
                world: &'w World,
                state: &'s mut Self::State,
            ) -> Self::Arg<'w, 's> {
                let ($($param,)*) = state;

                ($($param::arg(world, $param),)*)
            }

            fn flush(
                world: &mut World, 
                state: &mut Self::State
            ) {
                let ($($param,)*) = state;

                $(
                    $param::flush(world, $param);
                )*
            }
        }
    }
}

impl_param_tuple!();
impl_param_tuple!(P1);
impl_param_tuple!(P1, P2);
impl_param_tuple!(P1, P2, P3);
impl_param_tuple!(P1, P2, P3, P4);
impl_param_tuple!(P1, P2, P3, P4, P5);
impl_param_tuple!(P1, P2, P3, P4, P5, P6);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7);