use std::marker::PhantomData;

use crate::{
    store::Store, 
    schedule::{SystemMeta, UnsafeStore},
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

    fn init(&mut self, meta: &mut SystemMeta, world: &mut Store) {
        self.state = Some(F::Param::init(meta, world));
    }

    unsafe fn run_unsafe(&mut self, world: &UnsafeStore) -> Self::Out {
        let arg = F::Param::arg(
            world,
            self.state.as_mut().unwrap(),
        );

        self.fun.run(arg)
    }

    fn flush(&mut self, world: &mut Store) {
        F::Param::flush(world, self.state.as_mut().unwrap());
    }
}    

// struct IsFun;
impl<F, M:'static> IntoSystem<F::Out, fn(M,IsFun)> for F
where
    F: Fun<M> + Send + Sync + 'static,
    M: Send + Sync,
{
    type System = FunctionSystem<F, M>;

    fn into_system(this: Self) -> Self::System {
        FunctionSystem {
            fun: this,
            state: None,
            marker: Default::default()
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
        store::Store, 
        system::IntoSystem,
        schedule::{Schedule, SystemMeta, UnsafeStore},
    };

    use super::Param;

    static mut G_VALUE : Option<String> = None;

    #[test]
    fn arg_tuples() {
        let mut world = Store::new();

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
    }

    fn system<M>(world: &mut Store, fun: impl IntoSystem<(),M>)->String {
        set_global("init".to_string());
        let mut schedule = Schedule::new();
        schedule.add_system(fun);
        schedule.tick(world).unwrap();

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
        todo!();
        /*
        unsafe { 
            match &G_VALUE {
                Some(value) => String::from(value),
                None => panic!("no value")
            }
        }
        */
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
            _world: &'w UnsafeStore,
            _state: &'s mut Self::State,
        ) -> Self::Arg<'w, 's> {
            Self {
                name: type_name::<V>().to_string(),
                marker: PhantomData,
            }
        }

        fn init(_meta: &mut SystemMeta, _world: &mut Store) -> Self::State {
        }
    }
 }