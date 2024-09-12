use crate::{
    error::Result,
    store::Store, 
    schedule::{SystemMeta, UnsafeStore}
};

//
// Param
//
 
pub trait Param {
    type Arg<'s, 'l>: Param<Local = Self::Local>;
    type Local: Send + Sync + 'static;

    fn init(meta: &mut SystemMeta, store: &mut Store) -> Result<Self::Local>;

    fn arg<'s, 'l>(
        store: &'s UnsafeStore,
        local: &'l mut Self::Local, 
    ) -> Result<Self::Arg<'s, 'l>>;

    #[allow(unused)]
    fn flush(store: &mut Store, local: &mut Self::Local) {
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
            type Local = ($(<$param as Param>::Local,)*);

            fn init(
                meta: &mut SystemMeta,
                world: &mut Store, 
            ) -> Result<Self::Local> {
                Ok(($($param::init(meta, world)?,)*))
            }

            fn arg<'w, 's>(
                world: &'w UnsafeStore,
                state: &'s mut Self::Local,
            ) -> Result<Self::Arg<'w, 's>> {
                let ($($param,)*) = state;

                Ok(($($param::arg(world, $param)?,)*))
            }

            fn flush(
                world: &mut Store, 
                state: &mut Self::Local
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
    type Local = ();

    fn init(_meta: &mut SystemMeta, _store: &mut Store) -> Result<Self::Local> {
        Ok(())
    }

    fn arg<'w, 's>(
        _store: &'w UnsafeStore, 
        _local: &'s mut Self::Local,
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

#[cfg(test)]
mod test {
    use std::{any::type_name, marker::PhantomData};

    use essay_ecs_core_macros::Param;

    use crate::{
        error::Result,
        param::{Param, Res}, 
        schedule::{SystemMeta, UnsafeStore}, 
        Store
    };

    mod ecs { pub mod core { pub use crate::*; }}
    use ecs::{self as essay_ecs};
    
    use crate::core_app::CoreApp;

    #[test]
    fn test_param_arg() {
        let mut app = CoreApp::new();

        assert_eq!(
            "u8",
            app.eval(|param: TestArg<u8>| {
                param.name
            }).unwrap()
        );
    }

    #[test]
    fn test_derive_param_null() {
        let mut app = CoreApp::new();

        assert_eq!(
            "hello",
            app.eval(|param: NullParam| {
                param.message()
            }).unwrap()
        );
    }

    #[test]
    fn test_derive_param_resource() -> Result<()> {
        let mut app = CoreApp::new();

        assert_eq!(
            "resource is unassigned: alloc::string::String\n\tin essay_ecs_core::param::param::test::test_derive_param_resource::{{closure}}",
            app.eval(|param: ResourceParam| {
                param.value()
            }).unwrap_err().message()
        );

        app.insert_resource("string-resource".to_string());

        assert_eq!(
            "string-resource",
            app.eval(|param: ResourceParam| {
                param.value()
            })?
        );

        Ok(())
    }

    #[derive(Debug)]
    struct TestArg<V> {
        name: String,
        marker: PhantomData<V>,
    }

    impl<V> Param for TestArg<V> {
        type Arg<'w, 's> = TestArg<V>;
        type Local = ();

        fn arg<'w, 's>(
            _store: &'w UnsafeStore,
            _state: &'s mut Self::Local,
        ) -> Result<Self::Arg<'w, 's>> {
            Ok(Self {
                name: type_name::<V>().to_string(),
                marker: PhantomData,
            })
        }

        fn init(_meta: &mut SystemMeta, _store: &mut Store) -> Result<Self::Local> {
            Ok(())
        }
        
        fn flush(_store: &mut Store, _state: &mut Self::Local) {
            
        }
    }
 
    #[derive(Param)]
    struct NullParam {}

    impl NullParam {
        fn message(&self) -> String {
            "hello".to_string()
        }
    }

    #[derive(Param)]
    struct ResourceParam<'w> {
        string: Res<'w, String>,
    }

    impl ResourceParam<'_> {
        fn value(&self) -> String {
            self.string.get().clone()
        }
    }

    /*
    const _: () = {
    struct __PState<'w, 's> {
        v0: <Res::<'w, String> as Param>::State,
        marker: PhantomData<(&'w u8, &'s u8)>,
    }

    fn new_pstate<'w, 's>(meta: &mut SystemMeta, store: &mut Store) -> __PState<'w, 's> {
        __PState {
            v0: <Res::<'w, String> as Param>::init(meta, store),
            marker: PhantomData::default(),
        }
    }

    //impl<'w1> Param for ResourceParam<'w1> {
    impl Param for ResourceParam<'_> {
        type Arg<'w, 's> = ResourceParam<'w>;
        type State = __PState<'static, 'static>;

        fn arg<'w, 's>(
            store: &'w UnsafeStore,
            state: &'s mut Self::State,
        ) -> Result<Self::Arg<'w, 's>> {
            //let v0: Res<'w, String> = <Res<String> as Param>::Arg::<'w, 's>::arg(store, &mut state.v0)?;
            //let v0: Res<'w, String> = <Res<'w, String> as Param>::Arg::<'w, 's>::arg(store, &mut state.v0)?;

            Ok(ResourceParam {
                string: <Res<'w, String> as Param>::Arg::<'w, 's>::arg(store, &mut state.v0)?,
            })
        }

        fn init(meta: &mut SystemMeta, store: &mut Store) -> Self::State {
            new_pstate(meta, store)
        }
        
        fn flush(_store: &mut ecs::Store, _state: &mut Self::State) {
            
        }
    }
};
*/

}