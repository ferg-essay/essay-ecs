use std::marker::PhantomData;

use crate::{
    world::World, 
    schedule::{SystemMeta, IntoSystem, System}, 
    systems::{Param, Arg}, entity::Component
};

trait MapFun<R> {
    type Item:Component;
    type Param:Param;
}
/*
pub struct MapIterator<'a, 'w, F, R>
where
    F: MapFun<R> + 'a
{
    world: &'w World,
    state: &'a <F::Param as Param>::State,
    iter: &'a mut Box<dyn Iterator<Item=F::Item>>,
    map: &'a Box<dyn FnMut(F::Item, Arg<F::Param>)->R>,

    //marker: PhantomData<R>
}

impl<'a, 'w, F, R> Iterator for MapIterator<'a, 'w, F, R>
where
    F: MapFun<R>
{
    type Item = R;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) => {
                //let arg : Arg<F::Param> = F::Param::arg(self.world, &mut self.state);

                //Some((self.map)(item, arg))
                None
            }

            None => None,
        }
    }
}
*/
fn test() {
    let v = vec![1, 2, 3];
}

//
// eval function
//
/*
macro_rules! impl_system_function {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F, R, $($param: Param,)*> MapFun<R, fn($($param,)*)> for F
        where F:FnMut($($param,)*) -> R +
            FnMut($(Arg<$param>,)*) -> R,
        {
            type Param = ($($param,)*);

            fn map<'w, 's>(
                self, 
                arg: <Self::Param as Param>::Arg<'w, 's>
            ) -> MapIterator<'w, R> {
                let ($($param,)*) = arg;

                let result : Option<R> = Some(self($($param,)*));
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
*/
#[cfg(test)]
mod tests {
    use std::any::type_name;
    use std::marker::PhantomData;

    use crate::{world::World, schedule::{IntoSystem, SystemMeta, Schedule}};

    use super::Param;

    static mut G_VALUE : Option<String> = None;

    #[test]
    fn arg_tuples() {
        let mut world = World::new();

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

    fn system<M>(world: &mut World, fun: impl IntoSystem<(),M>)->String {
        set_global("init".to_string());
        let mut schedule = Schedule::new();
        schedule.add_system(fun);
        schedule.run(world);

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

    impl<V> Param for TestArg<V> {
        type Arg<'w, 's> = TestArg<V>;
        type State = ();

        fn arg<'w, 's>(
            _world: &'w World,
            _state: &'s mut Self::State,
        ) -> Self::Arg<'w, 's> {
            Self {
                name: type_name::<V>().to_string(),
                marker: PhantomData,
            }
        }

        fn init(_meta: &mut SystemMeta, _world: &mut World) -> Self::State {
        }
    }
 }