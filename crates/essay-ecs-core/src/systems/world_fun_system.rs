use std::marker::PhantomData;

use crate::{world::World, schedule::{SystemMeta, IntoSystem, System}, prelude::Local};

use super::{param::{Param, Arg}};

// IsFun prevents collision
pub struct IsWorld;

//
// Param
//
 
pub trait ParamExcl {
    type Arg<'s>;
    type State;

    fn init(world: &mut World, meta: &mut SystemMeta) -> Self::State;

    fn arg<'s>(
        state: &'s mut Self::State, 
    ) -> Self::Arg<'s>;
}

pub type ArgExcl<'s, P> = <P as ParamExcl>::Arg<'s>;

//
// FunctionSystem - a system implemented by a function
// 

pub struct WorldFunSystem<F, R, M>
where
    F: WorldFun<R, M>
{
    fun: F,
    state: Option<<F::Params as ParamExcl>::State>,
    marker: PhantomData<(R, M)>,
}

pub trait WorldFun<R, M> {
    type Params: ParamExcl;

    fn run(&mut self, world: &mut World, arg: ArgExcl<Self::Params>) -> R;
}

//
// Implementation
//

impl<F, R:'static, M> System for WorldFunSystem<F, R, M>
where
    M: 'static,
    F: WorldFun<R, M> + 'static
{
    type Out = R;

    fn init(&mut self, meta: &mut SystemMeta, world: &mut World) {
        meta.set_exclusive();
        self.state = Some(F::Params::init(world, meta));
    }

    fn run(&mut self, world: &mut World) -> Self::Out {
        let arg = F::Params::arg(
            self.state.as_mut().unwrap(),
        );

        self.fun.run(world, arg)
    }

    unsafe fn run_unsafe(&mut self, _world: &World) -> Self::Out {
        panic!("can't run exclusive system in unsafe mode");
    }

    fn flush(&mut self, world: &mut World) {
    }
}    

impl<F:'static, R:'static, M:'static> IntoSystem<R,fn(M,IsWorld)> for F
where
    F: WorldFun<R,M>
{
    type System = WorldFunSystem<F, R, M>;

    fn into_system(this: Self) -> Self::System {
        WorldFunSystem {
            fun: this,
            state: None,
            marker: Default::default()
        }
    }
}

//
// Function matching
//

macro_rules! impl_excl_function {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F: 'static, R, $($param: ParamExcl,)*> WorldFun<R, fn(IsWorld,$($param,)*)> for F
        where F:FnMut(&mut World, $($param,)*) -> R +
            FnMut(&mut World, $(ArgExcl<$param>,)*) -> R
        {
            type Params = ($($param,)*);

            fn run(&mut self, world: &mut World, arg: ArgExcl<($($param,)*)>) -> R {
                let ($($param,)*) = arg;
                self(world, $($param,)*)
            }
        }
    }
}

impl_excl_function!();
impl_excl_function!(P1);
impl_excl_function!(P1, P2);
impl_excl_function!(P1, P2, P3);
impl_excl_function!(P1, P2, P3, P4);
impl_excl_function!(P1, P2, P3, P4, P5);
impl_excl_function!(P1, P2, P3, P4, P5, P6);
impl_excl_function!(P1, P2, P3, P4, P5, P6, P7);

//
// Local param
//

impl<'a, T:Default + 'static> ParamExcl for Local<'a, T> {
    type State = T;
    type Arg<'s> = Local<'s, T>;

    fn init(_world: &mut World, _meta: &mut SystemMeta) -> Self::State {
        // let exl = std::sync::Exclusive::new(T::default());
        T::default()
    }

    fn arg<'w, 's>(
        state: &'s mut Self::State, 
    ) -> Self::Arg<'s> {
        Local(state)
    }
}

//
// Param composed of tuples
//

macro_rules! impl_param_excl_tuple {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($param: ParamExcl,)*> ParamExcl for ($($param,)*)
        {
            type Arg<'s> = ($($param::Arg<'s>,)*);
            type State = ($(<$param as ParamExcl>::State,)*);

            fn init(
                world: &mut World, 
                meta: &mut SystemMeta
            ) -> Self::State {
                ($($param::init(world, meta),)*)
            }

            fn arg<'s>(
                state: &'s mut Self::State,
            ) -> Self::Arg<'s> {
                let ($($param,)*) = state;

                ($($param::arg($param),)*)
            }
        }
    }
}

impl_param_excl_tuple!();
impl_param_excl_tuple!(P1);
impl_param_excl_tuple!(P1, P2);
impl_param_excl_tuple!(P1, P2, P3);
impl_param_excl_tuple!(P1, P2, P3, P4);
impl_param_excl_tuple!(P1, P2, P3, P4, P5);
impl_param_excl_tuple!(P1, P2, P3, P4, P5, P6);
impl_param_excl_tuple!(P1, P2, P3, P4, P5, P6, P7);

#[cfg(test)]
mod tests {
    use std::any::type_name;
    use std::marker::PhantomData;

    use crate::{world::World, prelude::Local, 
        schedule::{IntoSystem, SystemMeta, System}};

    use super::ParamExcl;

    static mut G_VALUE : Option<String> = None;

    #[test]
    fn arg_tuples() {
        let mut world = World::new();

        world.eval(|w: &mut World, l: Local<bool>| println!("world!"));
        /*
        set_global("init".to_string());
        system(&mut world, test_null);
        assert_eq!(get_global(), "test-null");
        system(&mut world, test_arg1);
        assert_eq!(get_global(), "test-arg1 u8");
        system(&mut world, test_arg2);
        assert_eq!(get_global(), "test-arg2 u8 u16");
        system(&mut world, test_arg3);
        assert_eq!(get_global(), "test-arg3 u8 u16 u32");
        system(&mut world, test_arg4);
        assert_eq!(get_global(), "test-arg4 u8 u16 u32 u64");
        system(&mut world, test_arg5);
        assert_eq!(get_global(), "test-arg5 u8 u16 u32 u64 i8");
        system(&mut world, test_arg6);
        assert_eq!(get_global(), "test-arg6 u8 u16 u32 u64 i8 i16");
        system(&mut world, test_arg7);
        assert_eq!(get_global(), "test-arg7 u8 u16 u32 u64 i8 i16 i32");
        */
    }

    fn system<R, M>(world: &mut World, fun: impl IntoSystem<R, M>)->String {
        set_global("init".to_string());
        let mut system = IntoSystem::into_system(fun);
        system.init(&mut SystemMeta::empty(), world);
        system.run(world);

        get_global()
    }

    fn test_null() {
       set_global("test-null".to_string());
    }

    fn test_arg1(arg1: TestArg<u8>) {
        set_global(format!("test-arg1 {}", arg1.name)); 
    }

    fn test_arg2(arg1: TestArg<u8>, arg2: TestArg<u16>) {
        set_global(format!("test-arg2 {} {}", arg1.name, arg2.name)); 
    }

    fn test_arg3(arg1: TestArg<u8>, arg2: TestArg<u16>, arg3: TestArg<u32>) {
        set_global(format!("test-arg3 {} {} {}", arg1.name, arg2.name, arg3.name)); 
    }

    fn test_arg4(arg1: TestArg<u8>, arg2: TestArg<u16>, arg3: TestArg<u32>, arg4: TestArg<u64>) {
        set_global(format!("test-arg4 {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name)); 
    }

    fn test_arg5(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>
    ) {
        set_global(format!("test-arg5 {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name)); 
    }

    fn test_arg6(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>,
    ) {
        set_global(format!("test-arg6 {} {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name, arg6.name)); 
    }

    fn test_arg7(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>, arg7: TestArg<i32>,
    ) {
        set_global(format!("test-arg7 {} {} {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name, arg6.name,
            arg7.name)); 
    }

    fn set_global(value: String) {
        unsafe { G_VALUE = Some(value); }
    }

    fn get_global() -> String {
        unsafe { 
            match &G_VALUE {
                Some(value) => String::from(value),
                None => panic!("no value")
            }
        }
    }

    #[derive(Debug)]
    struct TestArg<V> {
        name: String,
        marker: PhantomData<V>,
    }

    impl<V> ParamExcl for TestArg<V> {
        type Arg<'s> = TestArg<V>;
        type State = ();

        fn arg<'w, 's>(
            _state: &'s mut Self::State,
        ) -> Self::Arg<'s> {
            Self {
                name: type_name::<V>().to_string(),
                marker: PhantomData,
            }
        }

        fn init(_world: &mut World, _meta: &mut SystemMeta) -> Self::State {
            ()
        }
    }
 }