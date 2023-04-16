use std::{marker::PhantomData, ops::{DerefMut, Deref}};

use crate::{
    world::World, 
    prelude::{Param},
    schedule::{IntoSystem, System, SystemMeta}, 
    entity::{
        View, ViewBuilder, ViewCursor, Insert, InsertBuilder, InsertCursor
    }};

use super::param::Arg;

pub trait Channel {
    type In<'a>;
    type Out<'a>;

    fn new_in(&mut self) -> InComponent<Self>;
    fn new_out(&mut self) -> OutComponent<Self>;
}

pub struct In<'a, C:Channel>(C::In<'a>);

pub trait InChannel {
    type Channel:Channel;

    fn get_arg<'a>(&'a mut self, world: &'a World) -> <Self::Channel as Channel>::In<'a>;
}

type InComponent<C> = Box<dyn InChannel<Channel=C>>;

pub struct Out<'a, C:Channel>(C::Out<'a>);

pub trait OutChannel {
    type Channel:Channel;

    fn get_arg<'a>(&'a mut self, world: &'a World) -> <Self::Channel as Channel>::Out<'a>;
}

type OutComponent<C> = Box<dyn OutChannel<Channel=C>>;

//
// In implementation
//

impl<'a, C:Channel> Deref for In<'a, C> {
    type Target = C::In<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, C:Channel> DerefMut for In<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T:'static> Insert for InComponent<T> {
    fn build(builder: &mut InsertBuilder) {
        builder.add_column::<InComponent<T>>();
    }

    unsafe fn insert(cursor: &mut InsertCursor, value: Self) {
        cursor.insert(value);
    }
}

impl<C:'static> View for InComponent<C> {
    type Item<'t> = &'t mut InComponent<C>;

    fn build(builder: &mut ViewBuilder) {
        builder.add_ref::<InComponent<C>>();
    }

    unsafe fn deref<'a, 't>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t> {
        cursor.deref_mut::<InComponent<C>>()
    }
}

//
// Out implementation
//

impl<'a, C:Channel> Deref for Out<'a, C> {
    type Target = C::Out<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, C:Channel> DerefMut for Out<'a, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T:'static> Insert for OutComponent<T> {
    fn build(builder: &mut InsertBuilder) {
        builder.add_column::<OutComponent<T>>();
    }

    unsafe fn insert(cursor: &mut InsertCursor, value: Self) {
        cursor.insert(value);
    }
}

impl<C:'static> View for OutComponent<C> {
    type Item<'t> = &'t mut OutComponent<C>;

    fn build(builder: &mut ViewBuilder) {
        builder.add_ref::<OutComponent<C>>();
    }

    unsafe fn deref<'a, 't>(cursor: &mut ViewCursor<'a, 't>) -> Self::Item<'t> {
        cursor.deref_mut::<OutComponent<C>>()
    }
}

//
// System implementation for fun(Each, In), fun(Each, Out)
//

pub trait EachInFun<M> {
    type Item<'w>: View;
    type Channel: Channel;
    type Params: Param;

    fn run<'a,'w>(
        &mut self, 
        item: <Self::Item<'w> as View>::Item<'w>, // <'a>, 
        input: In<Self::Channel>,
        args: Arg<Self::Params>
    );
}

pub struct EachInSystem<M, F>
where
    F: EachInFun<M>
{
    fun: F,
    state: Option<<F::Params as Param>::State>,
    marker: PhantomData<M>,
}

impl<M, F:'static> System for EachInSystem<M, F>
where
    M: 'static,
    F: EachInFun<M>
{
    type Out = ();
    
    fn init(&mut self, meta: &mut SystemMeta, world: &mut World) {
        self.state = Some(F::Params::init(world, meta));
    }

    unsafe fn run_unsafe<'w>(&mut self, world: &World) {
        for (item, 
             input) 
            in world.view::<(F::Item<'_>,InComponent<F::Channel>)>() {
            let input = In(input.get_arg(world));

            let args = F::Params::arg(
                world,
                self.state.as_mut().unwrap(),
            );

            self.fun.run(item, input, args);
        }
    }
    
    fn flush(&mut self, world: &mut World) {
        F::Params::flush(world, self.state.as_mut().unwrap());
    }
}    
struct IsEachIn;

impl<F,M:'static> IntoSystem<(),fn(M,IsEachIn)> for F
where
    F: EachInFun<M> + 'static
{
    type System = EachInSystem<M, F>;

    fn into_system(this: Self) -> Self::System {
        EachInSystem {
            fun: this,
            state: None,
            marker: PhantomData,
        }
    }
}

macro_rules! impl_each_in_params {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F:'static, C:Channel, T:View, $($param: Param),*> 
        EachInFun<fn(T, C, $($param,)*)> for F
        where for<'w> F:FnMut(T, In<C>, $($param),*) -> () +
            FnMut(T::Item<'w>, In<C>, $(Arg<$param>),*) -> ()
        {
            type Item<'w> = T;
            type Channel = C;
            type Params = ($($param,)*);

            fn run<'b,'w>(
                &mut self, 
                item: T::Item<'w>, 
                input: In<Self::Channel>,
                arg: Arg<($($param,)*)>
            ) {
                let ($($param,)*) = arg;
                self(item, input, $($param,)*)
            }
        }
    }
}

impl_each_in_params!();
impl_each_in_params!(P1);
impl_each_in_params!(P1, P2);
impl_each_in_params!(P1, P2, P3);
impl_each_in_params!(P1, P2, P3, P4);
impl_each_in_params!(P1, P2, P3, P4, P5);
impl_each_in_params!(P1, P2, P3, P4, P5, P6);
impl_each_in_params!(P1, P2, P3, P4, P5, P6, P7);

//
// EachOut (item, Out)
//

pub trait EachOutFun<M> {
    type Item<'w>: View;
    type Channel: Channel;
    type Params: Param;

    fn run<'a,'w>(&mut self, 
        item: <Self::Item<'w> as View>::Item<'w>, // <'a>, 
        out: Out<Self::Channel>,
        args: Arg<Self::Params>
    );
}

pub struct EachOutSystem<M, F>
where
    F: EachOutFun<M>
{
    fun: F,
    state: Option<<F::Params as Param>::State>,
    marker: PhantomData<M>,
}

impl<M, F:'static> System for EachOutSystem<M, F>
where
    M: 'static,
    F: EachOutFun<M>
{
    type Out = ();

    fn init(&mut self, meta: &mut SystemMeta, world: &mut World) {
        self.state = Some(F::Params::init(world, meta));
    }

    unsafe fn run_unsafe<'w>(&mut self, world: &World) {
        for (item, 
             out) 
            in world.view::<(F::Item<'_>,OutComponent<F::Channel>)>() {
            let out = Out(out.get_arg(world));

            let args = F::Params::arg(
                world,
                self.state.as_mut().unwrap(),
            );
    
            self.fun.run(item, out, args);
        }
    }

    fn flush(&mut self, world: &mut World) {
        F::Params::flush(world, self.state.as_mut().unwrap());
    }
}

struct IsEachOut;

impl<F:'static,M:'static> IntoSystem<(),fn(M,IsEachOut)> for F
where
    F: EachOutFun<M>
{
    type System = EachOutSystem<M, F>;

    fn into_system(this: Self) -> Self::System {
        EachOutSystem {
            fun: this,
            state: None,
            marker: PhantomData,
        }
    }
}

macro_rules! impl_each_out_params {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F:'static, C:Channel, T:View, $($param: Param),*> 
        EachOutFun<fn(T, C, $($param,)*)> for F
        where for<'w> F:FnMut(T, Out<C>, $($param),*) -> () +
            FnMut(T::Item<'w>, Out<C>, $(Arg<$param>),*) -> ()
        {
            type Item<'w> = T;
            type Channel = C;
            type Params = ($($param,)*);

            fn run<'b,'w>(
                &mut self, 
                item: T::Item<'w>, 
                out: Out<Self::Channel>,
                arg: Arg<($($param,)*)>
            ) {
                let ($($param,)*) = arg;
                self(item, out, $($param,)*)
            }
        }
    }
}

impl_each_out_params!();
impl_each_out_params!(P1);
impl_each_out_params!(P1, P2);
impl_each_out_params!(P1, P2, P3);
impl_each_out_params!(P1, P2, P3, P4);
impl_each_out_params!(P1, P2, P3, P4, P5);
impl_each_out_params!(P1, P2, P3, P4, P5, P6);
impl_each_out_params!(P1, P2, P3, P4, P5, P6, P7);

#[cfg(test)]
mod tests {
    use crate::{world::World, systems::channel_system::Out, app::prelude::Tick};

    use super::{In, Channel, InChannel, InComponent, OutComponent, OutChannel};

    use std::{rc::Rc, cell::RefCell};

    use essay_ecs_macros::Component;

    #[test]
    fn each_in() {
        let mut world = World::new();

        let in_values = Rc::new(RefCell::new(Vec::<String>::new()));

        let mut channel = TestChannel::new(in_values.clone()); 

        world.spawn((TestA(1), channel.new_in()));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        world.eval(move |t :&mut TestA, mut input: In<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            for item in input.iter() {
                ptr.borrow_mut().push(item);
            }
        });

        assert_eq!(take(&values), "TestA(1)");

        in_values.borrow_mut().push("value-a".to_string());
        in_values.borrow_mut().push("value-b".to_string());

        let ptr = values.clone();
        world.eval(move |t :&mut TestA, mut input: In<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            for item in input.iter() {
                ptr.borrow_mut().push(item);
            }
        });

        assert_eq!(take(&values), "TestA(1), value-a[2], value-b[2]");

        let ptr = values.clone();
        world.eval(move |t :&mut TestA, mut input: In<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            for item in input.iter() {
                ptr.borrow_mut().push(item);
            }
        });

        assert_eq!(take(&values), "TestA(1)");

        in_values.borrow_mut().push("value-c".to_string());

        app.update();
        assert_eq!(take(&values), "TestA(1), value-c[4]");
    }

    #[test]
    fn each_out() {
        let mut app = App::new();

        let values = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut channel = TestChannel::new(values.clone()); 

        app.spawn((TestA(1), channel.new_out()));

        app.add_system(move |t :&mut TestA, mut out: Out<TestChannel>| {
            out.send(format!("{:?}", t));
        });

        app.update();
        assert_eq!(take(&values), "TestA(1)[1]");

        app.update();
        assert_eq!(take(&values), "TestA(1)[2]");

        app.update();
        assert_eq!(take(&values), "TestA(1)[3]");
    }

    #[test]
    fn each_in_out() {
        let mut app = App::new();

        let in_values = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut channel = TestChannel::new(in_values.clone()); 

        app.spawn((TestA(1), channel.new_in()));
        app.spawn((TestA(2), channel.new_out()));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();

        app.add_system(move |t :&mut TestA, mut input: In<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            for item in input.iter() {
                ptr.borrow_mut().push(item);
            }
        });

        let ptr = values.clone();
        
        app.add_system(move |t :&mut TestA, mut out: Out<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            out.send(format!("send-{:?}", t));
        });

        app.update();
        assert_eq!(take(&values), "TestA(1), TestA(2)");

        app.update();
        assert_eq!(take(&values), "TestA(1), send-TestA(2)[1][2], TestA(2)");

        app.update();
        assert_eq!(take(&values), "TestA(1), send-TestA(2)[2][3], TestA(2)");

        app.update();
        assert_eq!(take(&values), "TestA(1), send-TestA(2)[3][4], TestA(2)");
    }

    fn take(values: &Rc<RefCell<Vec<String>>>) -> String {
        let v : Vec<String> = values.borrow_mut()
            .drain(..)
            .collect();

        v.join(", ")
    }

    #[derive(Component,PartialEq, Debug)]
    struct TestA(usize);
    #[derive(Debug)]
    struct TestInFiber(usize);

    struct TestChannel {
        values: Rc<RefCell<Vec<String>>>,
    }

    impl TestChannel {
        fn new(values: Rc<RefCell<Vec<String>>>) -> Self {
            Self {
                values: values,
            }
        }
    }

    impl Channel for TestChannel {
        type In<'a> = InChannelTestItem<'a>;
        type Out<'a> = OutChannelTestItem<'a>;

        fn new_in(&mut self) -> InComponent<Self> {
            Box::new(InChannelTest::new(self.values.clone()))
        }

        fn new_out(&mut self) -> OutComponent<Self> {
            Box::new(OutChannelTest::new(self.values.clone()))
        }
    }

    struct InChannelTest {
        values: Rc<RefCell<Vec<String>>>,
    }

    struct InChannelTestItem<'a> {
        fiber_in: &'a mut InChannelTest,
        tick: u64,
    }

    impl InChannelTest {
        fn new(values: Rc<RefCell<Vec<String>>>) -> Self {
            Self {
                values: values,
            }
        }

        fn new_item(&mut self, tick: u64) -> InChannelTestItem {
            InChannelTestItem::new(self, tick)
        }

        fn new_box(values: Rc<RefCell<Vec<String>>>) -> InComponent<TestChannel> {
            Box::new(Self::new(values))
        }
    }

    impl InChannel for InChannelTest {
        type Channel = TestChannel;

        fn get_arg(&mut self, world: &World) -> InChannelTestItem {
            self.new_item(world.resource::<Tick>().value())
        }
    }

    impl<'a> InChannelTestItem<'a> {
        fn new(fiber_in: &'a mut InChannelTest, tick: u64) -> Self {
            Self {
                fiber_in: fiber_in,
                tick: tick,
            }
        }

        fn iter(&mut self) -> Vec<String> {
            let values: Vec<String> = self.fiber_in.values.borrow_mut()
                .drain(..)
                .map(|s| format!("{}[{}]", s, self.tick))
                .collect();

            values
        }
    }

    struct OutChannelTest {
        values: Rc<RefCell<Vec<String>>>,
    }

    struct OutChannelTestItem<'a> {
        out: &'a mut OutChannelTest,
        tick: u64,
    }

    impl OutChannelTest {
        fn new(values: Rc<RefCell<Vec<String>>>) -> Self {
            Self {
                values: values,
            }
        }

        fn new_item(&mut self, tick: u64) -> OutChannelTestItem {
            OutChannelTestItem::new(self, tick)
        }

        fn new_box(values: Rc<RefCell<Vec<String>>>) -> OutComponent<TestChannel> {
            Box::new(Self::new(values))
        }
    }

    impl OutChannel for OutChannelTest {
        type Channel = TestChannel;

        fn get_arg(&mut self, world: &World) -> OutChannelTestItem {
            self.new_item(world.resource::<Tick>().value())
        }
    }

    impl<'a> OutChannelTestItem<'a> {
        fn new(out: &'a mut OutChannelTest, tick: u64) -> Self {
            Self {
                out: out,
                tick: tick,
            }
        }

        fn send(&mut self, value: String) {
            self.out.values.borrow_mut().push(format!("{}[{}]", value, self.tick));
        }
    }
}