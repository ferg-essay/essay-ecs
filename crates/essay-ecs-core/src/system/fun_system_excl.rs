use std::marker::PhantomData;

use crate::{
    error::Result,
    schedule::{SystemMeta, UnsafeStore},
    store::Store, 
    system::{IntoSystem, System}, Local,
};

// IsFun prevents collision
pub struct IsWorld;

//
// Param
//
 
pub trait ParamExcl {
    type Arg<'s>;
    type State: Send + Sync + 'static;

    fn init(world: &mut Store, meta: &mut SystemMeta) -> Result<Self::State>;

    fn arg<'s>(
        state: &'s mut Self::State, 
    ) -> Self::Arg<'s>;
}

pub type ArgExcl<'s, P> = <P as ParamExcl>::Arg<'s>;

//
// FunctionSystem - a system implemented by a function
// 

pub struct FunSystemExcl<F, R, M>
where
    F: FunExcl<R, M> + Send + Sync + 'static,
    R: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    fun: F,
    state: Option<<F::Params as ParamExcl>::State>,
    marker: PhantomData<(R, M)>,
}

pub trait FunExcl<R, M> : Send + Sync + 'static {
    type Params: ParamExcl;

    fn run(&mut self, world: &mut Store, arg: ArgExcl<Self::Params>) -> Result<R>;
}

//
// Implementation
//

impl<F, R:'static, M> System for FunSystemExcl<F, R, M>
where
    F: FunExcl<R, M> + Send + Sync + 'static,
    R: Send + Sync + 'static,
    M: Send + Sync + 'static,
{
    type Out = R;

    fn init(&mut self, meta: &mut SystemMeta, world: &mut Store) -> Result<()> {
        meta.set_exclusive();
        self.state = Some(F::Params::init(world, meta)?);

        Ok(())
    }

    fn run(&mut self, world: &mut UnsafeStore) -> Result<Self::Out> {
        let arg = F::Params::arg(
            self.state.as_mut().unwrap(),
        );

        self.fun.run(world, arg)
    }

    unsafe fn run_unsafe(&mut self, _world: &UnsafeStore) -> Result<Self::Out> {
        // TODO: location
        Err("can't run exclusive system in unsafe mode".into())
    }

    fn flush(&mut self, _world: &mut Store) {
    }
}    

impl<F:'static, R:'static, M:'static> IntoSystem<R, fn(M,IsWorld)> for F
where
    F: FunExcl<R,M> + Send + Sync + 'static,
    R: Send + Sync + 'static,
    M: Send + Sync + 'static
{
    type System = FunSystemExcl<F, R, M>;

    fn into_system(this: Self) -> Self::System {
        FunSystemExcl {
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
        impl<F, R, $($param: ParamExcl,)*> FunExcl<R, fn(IsWorld,$($param,)*)> for F
        where F:FnMut(&mut Store, $($param,)*) -> Result<R> + Send + Sync + 'static +
            FnMut(&mut Store, $(ArgExcl<$param>,)*) -> Result<R>,
            R: Send + Sync + 'static
        {
            type Params = ($($param,)*);

            fn run(&mut self, world: &mut Store, arg: ArgExcl<($($param,)*)>) -> Result<R> {
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
impl<'a, T: Default + Send + Sync + 'static> ParamExcl for Local<'a, T> {
    type State = T;
    type Arg<'s> = Local<'s, T>;

    fn init(_world: &mut Store, _meta: &mut SystemMeta) -> Result<Self::State> {
        Ok(T::default())
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
                world: &mut Store, 
                meta: &mut SystemMeta
            ) -> Result<Self::State> {
                meta.set_exclusive();

                Ok(($($param::init(world, meta)?,)*))
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

impl ParamExcl for () {
    type Arg<'s> = ();
    type State = ();

    fn init(_world: &mut Store, _meta: &mut SystemMeta) -> Result<Self::State> {
        Ok(())
    }

    fn arg<'s>(_state: &'s mut Self::State) -> Self::Arg<'s> {
        ()
    }
}

// impl_param_excl_tuple!();
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

    use crate::{
        error::Result,
        store::Store, 
        schedule::SystemMeta,
        system::IntoSystem, 
    };

    use super::ParamExcl;

    static mut _G_VALUE : Option<String> = None;

    #[test]
    fn arg_tuples() {
        //let mut core = CoreApp::new();
        //let mut schedule = Schedule::new();

        //core.run_system(|w: &mut World| println!("world!"));
        //core.run_system(|w: &mut Store, l: Local<bool>| println!("world!"));
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

    fn _system<R, M>(_world: &mut Store, _fun: impl IntoSystem<R, M>)->String {
        /*
        set_global("init".to_string());
        let mut system = IntoSystem::into_system(fun);
        system.init(&mut SystemMeta::empty(), world);
        system.run(world);

        get_global()
        */
        todo!();
    }

    fn _test_null() {
       _set_global("test-null".to_string());
    }

    fn _test_arg1(arg1: TestArg<u8>) {
        _set_global(format!("test-arg1 {}", arg1._name)); 
    }

    fn _test_arg2(arg1: TestArg<u8>, arg2: TestArg<u16>) {
        _set_global(format!("test-arg2 {} {}", arg1._name, arg2._name)); 
    }

    fn _test_arg3(arg1: TestArg<u8>, arg2: TestArg<u16>, arg3: TestArg<u32>) {
        _set_global(format!("test-arg3 {} {} {}", arg1._name, arg2._name, arg3._name)); 
    }

    fn _test_arg4(arg1: TestArg<u8>, arg2: TestArg<u16>, arg3: TestArg<u32>, arg4: TestArg<u64>) {
        _set_global(format!("test-arg4 {} {} {} {}",
            arg1._name, arg2._name, arg3._name, arg4._name)); 
    }

    fn _test_arg5(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>
    ) {
        _set_global(format!("test-arg5 {} {} {} {} {}",
            arg1._name, arg2._name, arg3._name, arg4._name, arg5._name)); 
    }

    fn _test_arg6(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>,
    ) {
        _set_global(format!("test-arg6 {} {} {} {} {} {}",
            arg1._name, arg2._name, arg3._name, arg4._name, arg5._name, arg6._name)); 
    }

    fn _test_arg7(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>, arg7: TestArg<i32>,
    ) {
        _set_global(format!("test-arg7 {} {} {} {} {} {} {}",
            arg1._name, arg2._name, arg3._name, arg4._name, arg5._name, arg6._name,
            arg7._name)); 
    }

    fn _set_global(value: String) {
        unsafe { _G_VALUE = Some(value); }
    }

    fn _get_global() -> String {
        todo!();
        /*
        unsafe { 
            match &_G_VALUE {
                Some(value) => String::from(value),
                None => panic!("no value")
            }
        }
        */
    }

    #[derive(Debug)]
    struct TestArg<V> {
        _name: String,
        marker: PhantomData<V>,
    }

    impl<V:Sync+Send+'static> ParamExcl for TestArg<V> {
        type Arg<'s> = TestArg<V>;
        type State = ();

        fn arg<'w, 's>(
            _state: &'s mut Self::State,
        ) -> Self::Arg<'s> {
            Self {
                _name: type_name::<V>().to_string(),
                marker: PhantomData,
            }
        }

        fn init(_world: &mut Store, _meta: &mut SystemMeta) -> Result<Self::State> {
            Ok(())
        }
    }
 }