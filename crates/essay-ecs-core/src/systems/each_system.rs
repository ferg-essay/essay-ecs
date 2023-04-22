use std::{marker::PhantomData};

use crate::{world::World, entity::{View, ComponentId}, 
    schedule::{System, IntoSystem, SystemMeta}
};

use super::{Param, param::Arg};

// IsEach prevents collisions
pub struct IsEach;

pub struct EachSystem<M, F>
where
    F: EachFun<M>
{
    fun: F,
    state: Option<<F::Params as Param>::State>,
    marker: PhantomData<M>,
}

pub trait EachFun<M> : Send + Sync {
    type Item<'w>: View;
    type Params: Param;

    fn run<'a,'w>(
        &mut self, 
        item: <Self::Item<'w> as View>::Item<'w>, // <'a>, 
        arg: Arg<Self::Params>
    );
}

//
// Implementation
//

impl<M, F:'static> EachSystem<M, F>
where
    F: EachFun<M>
{
    fn new<'w>(
        fun: F
    ) -> Self {
        Self {
            fun: fun,
            state: None,

            marker: PhantomData,
        }
    }
}

impl<M, F> System for EachSystem<M, F>
where
    M: Send + Sync + 'static,
    F: EachFun<M> + Send + Sync + 'static
{
    type Out = ();
    
    fn init(&mut self, meta: &mut SystemMeta, world: &mut World) {
        let plan = world.view_build::<F::Item<'_>>();

        for id in plan.components() {
            meta.insert_component(ComponentId::from(*id));
        }

        for id in plan.mut_components() {
            meta.insert_component_mut(ComponentId::from(*id));
        }
        
        //F::Item::init(meta);
        self.state = Some(F::Params::init(meta, world))
    }
    
    unsafe fn run_unsafe<'w>(&mut self, world: &World) {
        for entity in world.view::<F::Item<'_>>() {
            let args = F::Params::arg(
                world,
                self.state.as_mut().unwrap(),
            );
    
            self.fun.run(entity, args);
        }
    }
    
    fn flush(&mut self, world: &mut World) {
        F::Params::flush(world, self.state.as_mut().unwrap());
    }
}

impl<F:'static, M:'static> IntoSystem<(), fn(M,IsEach)> for F
where
    M: Send + Sync,
    F: EachFun<M>
{
    type System = EachSystem<M, F>;

    fn into_system(this: Self) -> Self::System {
        EachSystem::new(this)
    }
}

//
// EachFun: function system matching
//
pub struct IsPlain;

macro_rules! impl_each_function {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F:'static, T:View, $($param: Param),*> EachFun<fn(IsPlain, T, $($param,)*)> for F
        where for<'w> F:FnMut(T, $($param),*) -> () + Send + Sync +
            FnMut(T::Item<'w>, $(Arg<$param>),*) -> ()
        {
            type Item<'w> = T;
            type Params = ($($param,)*);

            fn run<'b,'w>(
                &mut self, 
                item: T::Item<'b>, 
                arg: Arg<($($param,)*)>
            ) {
                let ($($param,)*) = arg;
                self(item, $($param,)*)
            }
        }
    }
}

impl_each_function!();
impl_each_function!(P1);
impl_each_function!(P1, P2);
impl_each_function!(P1, P2, P3);
impl_each_function!(P1, P2, P3, P4);
impl_each_function!(P1, P2, P3, P4, P5);
impl_each_function!(P1, P2, P3, P4, P5, P6);
impl_each_function!(P1, P2, P3, P4, P5, P6, P7);

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use crate::{World, entity::Component};

    #[test]
    fn test_each() {
        let mut world = World::new();

        world.spawn(TestA(1));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        //app.add_system(system_each_ref);
        /*
        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("{:?}", t));
        });

        world.spawn(TestA(2));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("{:?}", t));
        });
        assert_eq!(take(&values), "TestA(1), TestA(2)");

        world.spawn((TestA(3), TestB(4)));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("{:?}", t));
        });
        assert_eq!(take(&values), "TestA(1), TestA(2)");
        */
    }

    #[test]
    fn test_each_ref() {
        let mut world = World::new();

        world.spawn(TestA(1));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        //app.add_system(system_each_ref);
        /*
        let ptr = values.clone();
        world.eval(move |t :&TestA| {
            ptr.borrow_mut().push(format!("{:?}", t));
        });

        assert_eq!(take(&values), "TestA(1)");
        */
    }

    #[test]
    fn test_each_a_b() {
        let mut world = World::new();
        /*
        world.spawn(TestA(1));
        world.spawn(TestB(2));
        world.spawn((TestA(3),TestB(4)));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));
        let ptr = values.clone();

        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("a-{:?}", t));
        });

        let ptr = values.clone();
        world.eval(move |t :&mut TestB| {
            ptr.borrow_mut().push(format!("b-{:?}", t));
        });

        assert_eq!(take(&values), "a-TestA(1), a-TestA(3), b-TestB(2), b-TestB(4)");
        */
    }

    #[test]
    fn test_each_tuple() {
        let mut world = World::new();
        /*
        world.spawn(TestA(1));
        world.spawn(TestB(2));
        world.spawn((TestA(3),TestB(4)));
        world.spawn((TestB(5),TestA(6)));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));
        let ptr = values.clone();

        world.eval(move |a:(&TestA, &TestB)| {
            ptr.borrow_mut().push(format!("{:?}", a));
        });

        assert_eq!(take(&values), "(TestA(3), TestB(4)), (TestA(6), TestB(5))");
        */
    }

    #[test]
    fn test_each_tuple_rev() {
        let mut world = World::new();
        /*
        world.spawn(TestA(1));
        world.spawn(TestB(2));
        world.spawn((TestA(3),TestB(4)));
        world.spawn((TestB(5),TestA(6)));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));
        let ptr = values.clone();

        world.eval(move |a:(&TestB, &TestA)| {
            ptr.borrow_mut().push(format!("{:?}", a));
        });

        assert_eq!(take(&values), "(TestB(4), TestA(3)), (TestB(5), TestA(6))");
        */
    }

    #[test]
    fn test_each_mut() {
        /*
        let mut world = World::new();

        world.spawn(TestA(0));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            t.0 += 1;
            ptr.borrow_mut().push(format!("{:?}", t));
        });

        assert_eq!(take(&values), "TestA(1)");

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            t.0 += 1;
            ptr.borrow_mut().push(format!("{:?}", t));
        });
        assert_eq!(take(&values), "TestA(2)");

        world.spawn(TestA(0));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            t.0 += 1;
            ptr.borrow_mut().push(format!("{:?}", t));
        });
        assert_eq!(take(&values), "TestA(3), TestA(1)");

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            t.0 += 1;
            ptr.borrow_mut().push(format!("{:?}", t));
        });
        assert_eq!(take(&values), "TestA(4), TestA(2)");
        */
    }

    #[test]
    fn test_two_each() {
        /*
        let mut world = World::new();

        world.spawn(TestA(0));
        world.spawn(TestB(0));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("S-A {:?}", t));
        });

        let ptr = values.clone();
        world.eval(move |t :&mut TestB| {
            ptr.borrow_mut().push(format!("S-B {:?}", t));
        });

        assert_eq!(take(&values), "S-A TestA(0), S-B TestB(0)");

        world.spawn(TestA(0));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA| {
            ptr.borrow_mut().push(format!("S-A {:?}", t));
        });

        let ptr = values.clone();
        world.eval(move |t :&mut TestB| {
            ptr.borrow_mut().push(format!("S-B {:?}", t));
        });

        assert_eq!(take(&values), "S-A TestA(0), S-A TestA(0), S-B TestB(0)");
        */
    }

    #[test]
    fn test_each_res() {
        /*
        let mut app = App::new();
        app.spawn(TestA(0));
        app.add_resource("hello".to_string());
        */
        /*
        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(move |t :&mut TestA, name: TestArg<String>| {
            ptr.borrow_mut().push(format!("S-A {:?} {:?}", t, name.name));
        });

        app.update();
        assert_eq!(take(&values), "S-A TestA(0) \"alloc::string::String\"");
        */
    }

    #[test]
    fn test_each_in() {
        /*
        let mut app = App::new();
        let ent_ref = app.spawn(TestA(1));
        ent_ref.push(&mut app, TestInFiber(2));


        // app.add_system(system_each_in);

        app.update();

        app.spawn(TestA(3));
        
        app.update();
        // assert_eq!(take(&values), "S-A TestA(0) \"alloc::string::String\"");
        */
    }
    /*
    fn system_each_in(test: &mut TestA, input: In<TestFiber>) {
        println!("system-each-in {:?} {:?}", test, Deref::deref(&input));
    }
    */
    /*
    fn system_each_ref(test: &mut TestA) {
        println!("system-each {:?}", test);
    }
    */

    fn take(values: &Rc<RefCell<Vec<String>>>) -> String {
        let v : Vec<String> = values.borrow_mut().drain(..).collect();

        v.join(", ")
    }

    #[derive(PartialEq, Debug)]
    struct TestA(u32);

    impl Component for TestA {}

    #[derive(PartialEq, Debug)]
    struct TestB(u32);

    impl Component for TestB {}
}