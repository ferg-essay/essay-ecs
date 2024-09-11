use crate::{
    error::Result,
    store::Store, 
    schedule::{SystemMeta, UnsafeStore}
};

//
// Param
//
 
pub trait Param {
    type Arg<'w, 's>: Param<State = Self::State>;
    type State: Send + Sync + 'static;

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Self::State;

    fn arg<'w, 's>(
        store: &'w UnsafeStore,
        state: &'s mut Self::State, 
    ) -> Result<Self::Arg<'w, 's>>;

    #[allow(unused)]
    fn flush(store: &mut Store, state: &mut Self::State) {
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
                world: &mut Store, 
            ) -> Self::State {
                ($($param::init(meta, world),)*)
            }

            fn arg<'w, 's>(
                world: &'w UnsafeStore,
                state: &'s mut Self::State,
            ) -> Result<Self::Arg<'w, 's>> {
                let ($($param,)*) = state;

                Ok(($($param::arg(world, $param)?,)*))
            }

            fn flush(
                world: &mut Store, 
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

impl Param for ()
{
    type Arg<'w, 's> = ();
    type State = ();

    fn init(_meta: &mut SystemMeta, _world: &mut Store) -> Self::State {
        ()
    }

    fn arg<'w, 's>(
        _world: &'w UnsafeStore, 
        _state: &'s mut Self::State,
    ) -> Result<Self::Arg<'w, 's>> {
        Ok(())
    }
}

// impl_param_tuple!();
impl_param_tuple!(P1);
impl_param_tuple!(P1, P2);
impl_param_tuple!(P1, P2, P3);
impl_param_tuple!(P1, P2, P3, P4);
impl_param_tuple!(P1, P2, P3, P4, P5);
impl_param_tuple!(P1, P2, P3, P4, P5, P6);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_param_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);
