use std::{any::type_name, marker::PhantomData};

use crate::{
    error::Result,
    schedule::{SystemMeta, UnsafeStore},
    store::Store, 
    system::{IntoSystem, System},
};

use crate::param::{Param, Arg};

// IsFun prevents collision
pub struct IsFun;

//
// FunctionSystem - a system implemented by a function
// 

pub struct FunctionSystem<F, M>
where
    F: Fun<M>
{
    fun: F,
    state: Option<<F::Param as Param>::State>,

    name: String,
    marker: PhantomData<fn() -> M>,
}

pub trait Fun<M>: Send + Sync + 'static {
    type Param: Param;
    type Out;

    fn run(&mut self, arg: Arg<Self::Param>) -> Self::Out;
}

//
// Implementation
//

impl<F, M> System for FunctionSystem<F, M>
where
    M: 'static,
    F: Fun<M> + Send + Sync + 'static
{
    type Out = F::Out;

    fn init(&mut self, meta: &mut SystemMeta, store: &mut Store) {
        self.state = Some(F::Param::init(meta, store));
    }

    unsafe fn run_unsafe(&mut self, store: &UnsafeStore) -> Result<Self::Out> {
        // Ok(self.fun.run(F::Param::arg(store, self.state.as_mut().unwrap())?))

        match F::Param::arg(store, self.state.as_mut().unwrap()) {
            Ok(arg) => {
                Ok(self.fun.run(arg))
            }
            Err(err) => {
                Err(format!("{}\n\tin {}", err.message(), self.name).into())
            }
        }
    }

    fn flush(&mut self, store: &mut Store) {
        F::Param::flush(store, self.state.as_mut().unwrap());
    }
}    

// struct IsFun;
impl<F, M:'static> IntoSystem<F::Out, fn(M,IsFun)> for F
where
    F: Fun<M> + Send + Sync + 'static,
    M: Send + Sync,
{
    type System = FunctionSystem<F, M>;

    #[track_caller]
    fn into_system(this: Self) -> Self::System {
        FunctionSystem {
            fun: this,
            state: None,

            name: type_name::<F>().to_string(),
            marker: Default::default(),
        }
    }
}

//
// Function matching
//

macro_rules! impl_system_function {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, R, $($param: Param,)*> Fun<fn($($param,)*) -> R> for F
        where F:FnMut($($param,)*) -> R + Send + Sync + 'static +
            FnMut($(Arg<$param>,)*) -> R,
        {
            type Out = R;
            type Param = ($($param,)*);

            fn run(&mut self, arg: Arg<($($param,)*)>) -> R {
                let ($($param,)*) = arg;
                self($($param,)*)
            }
        }
    }
}

impl_system_function!();
impl_system_function!(P1);
impl_system_function!(P1, P2);
impl_system_function!(P1, P2, P3);
impl_system_function!(P1, P2, P3, P4);
impl_system_function!(P1, P2, P3, P4, P5);
impl_system_function!(P1, P2, P3, P4, P5, P6);
impl_system_function!(P1, P2, P3, P4, P5, P6, P7);
impl_system_function!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_system_function!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_system_function!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_system_function!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);

#[cfg(test)]
mod tests {
    use std::any::type_name;
    use std::marker::PhantomData;

    use crate::{
        core_app::{Core, CoreApp}, 
        error::Result,
        schedule::{SystemMeta, UnsafeStore}, 
        store::Store,
    };

    use super::Param;

    #[test]
    fn arg_tuples() -> Result<()> {
        let mut app = CoreApp::new();

        assert_eq!(
            app.eval(test_null)?,
            "test-null"
        );

        assert_eq!(
            app.eval(test_arg1)?,
            "test-arg1 u8"
        );

        assert_eq!(
            app.eval(test_arg2)?,
            "test-arg2 u8 u16"
        );

        assert_eq!(
            app.eval(test_arg3)?,
            "test-arg3 u8 u16 u32"
        );

        assert_eq!(
            app.eval(test_arg4)?,
            "test-arg4 u8 u16 u32 u64"
        );

        assert_eq!(
            app.eval(test_arg5)?,
            "test-arg5 u8 u16 u32 u64 i8"
        );

        assert_eq!(
            app.eval(test_arg6)?,
            "test-arg6 u8 u16 u32 u64 i8 i16"
        );

        assert_eq!(
            app.eval(test_arg7)?,
            "test-arg7 u8 u16 u32 u64 i8 i16 i32"
        );

        Ok(())
    }

    #[test]
    fn bogus_arg_in_eval() {
        let mut app = CoreApp::new();

        assert_eq!(
            app.eval(bogus_arg1).err().unwrap().message(),
            "bogus-arg\n\tin essay_ecs_core::system::fun_system::tests::bogus_arg1"
        );
    }

    #[test]
    fn bogus_arg_in_tick() {
        let mut app = CoreApp::new();

        app.system(Core, bogus_arg_null);
        //app.tick().unwrap();
        
        assert_eq!(
            app.tick().unwrap_err().message(),
            "BogusArg test internal arg error\n\tin essay_ecs_core::system::fun_system::tests::bogus_arg_null"
        );
    }

    fn test_null() -> String {
       "test-null".to_string()
    }

    fn test_arg1(arg1: TestArg<u8>) -> String {
        format!("test-arg1 {}", arg1.name)
    }

    fn test_arg2(arg1: TestArg<u8>, arg2: TestArg<u16>) -> String {
        format!("test-arg2 {} {}", arg1.name, arg2.name)
    }

    fn test_arg3(
        arg1: TestArg<u8>, 
        arg2: TestArg<u16>, 
        arg3: TestArg<u32>
    ) -> String {
        format!("test-arg3 {} {} {}", arg1.name, arg2.name, arg3.name)
    }

    fn test_arg4(
        arg1: TestArg<u8>, 
        arg2: TestArg<u16>, 
        arg3: TestArg<u32>, 
        arg4: TestArg<u64>
    ) -> String {
        format!("test-arg4 {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name)
    }

    fn test_arg5(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>
    ) -> String {
        format!("test-arg5 {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name)
    }

    fn test_arg6(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>,
    ) -> String {
        format!("test-arg6 {} {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name, arg6.name)
    }

    fn test_arg7(arg1: TestArg<u8>, arg2: TestArg<u16>,
        arg3: TestArg<u32>, arg4: TestArg<u64>,
        arg5: TestArg<i8>, arg6: TestArg<i16>, arg7: TestArg<i32>,
    ) -> String {
        format!("test-arg7 {} {} {} {} {} {} {}",
            arg1.name, arg2.name, arg3.name, arg4.name, arg5.name, arg6.name,
            arg7.name)
    }

    fn bogus_arg1(_arg: BogusArg) -> String {
        "bogus-arg".to_string()
    }

    fn bogus_arg_null(_arg: BogusArg) {
        println!("tick");
        panic!("Shouldn't execute here")
    }
 
    #[derive(Debug)]
    struct TestArg<V> {
        name: String,
        marker: PhantomData<V>,
    }

    impl<V> Param for TestArg<V> {
        type Arg<'w, 's> = TestArg<V>;
        type State = ();

        fn arg<'w, 's>(
            _store: &'w UnsafeStore,
            _state: &'s mut Self::State,
        ) -> Result<Self::Arg<'w, 's>> {
            Ok(Self {
                name: type_name::<V>().to_string(),
                marker: PhantomData,
            })
        }

        fn init(_meta: &mut SystemMeta, _store: &mut Store) -> Self::State {
        }
    }
 
    #[derive(Debug)]
    struct BogusArg {
    }

    impl Param for BogusArg {
        type Arg<'w, 's> = BogusArg;
        type State = ();

        fn arg<'w, 's>(
            _world: &'w UnsafeStore,
            _state: &'s mut Self::State,
        ) -> Result<Self::Arg<'w, 's>> {
            Err("BogusArg test internal arg error".into())
        }

        fn init(_meta: &mut SystemMeta, _world: &mut Store) -> Self::State {
        }
    }
}