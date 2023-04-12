use std::{marker::PhantomData, ops::{DerefMut, Deref}};

use crate::{world::prelude::World, prelude::{Param, IntoSystem, System}, entity::prelude::{Query, QueryBuilder, QueryCursor, Insert, InsertBuilder, InsertCursor}};

use super::param::Arg;

pub trait Channel {
    type In<'a>;
    type Out<'a>;
}

pub struct In<'a, C:Channel>(C::In<'a>);

pub trait InChannel {
    type Channel:Channel;

    fn get<'a>(&'a mut self, world: &'a World) -> <Self::Channel as Channel>::In<'a>;
}

type InComponent<C> = Box<dyn InChannel<Channel=C>>;

pub struct Out<'a, C:Channel>(C::Out<'a>);

pub trait OutChannel {
    type Channel:Channel;

    fn get<'a>(&'a mut self, world: &'a World) -> <Self::Channel as Channel>::Out<'a>;
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

impl<C:'static> Query for InComponent<C> {
    type Item<'t> = &'t mut InComponent<C>;

    fn build(builder: &mut QueryBuilder) {
        builder.add_ref::<InComponent<C>>();
    }

    unsafe fn query<'a,'t>(cursor: &mut QueryCursor<'a,'t>) -> Self::Item<'t> {
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

impl<C:'static> Query for OutComponent<C> {
    type Item<'t> = &'t mut OutComponent<C>;

    fn build(builder: &mut QueryBuilder) {
        builder.add_ref::<OutComponent<C>>();
    }

    unsafe fn query<'a,'t>(cursor: &mut QueryCursor<'a,'t>) -> Self::Item<'t> {
        cursor.deref_mut::<OutComponent<C>>()
    }
}

//
// System implementation for fun(Each, In), fun(Each, Out)
//

pub trait EachInFun<M> {
    type Item<'w>: Query;
    type Channel: Channel;
    type Params: Param;

    fn run<'a,'w>(&mut self, 
        world: &World<'w>,
        item: <Self::Item<'w> as Query>::Item<'w>, // <'a>, 
        input: In<Self::Channel>,
        args: Arg<Self::Params>
    );
}

pub struct EachInSystem<M, F>
where
    F: EachInFun<M>
{
    fun: F,
    marker: PhantomData<M>,
}

impl<M, F:'static> EachInSystem<M, F>
where
    F: EachInFun<M>
{
    fn new<'w>(_world: &mut World<'w>, fun: F) -> Self {
        Self {
            fun: fun,
            marker: PhantomData,
        }
    }
}

impl<M, F:'static> System for EachInSystem<M, F>
where
    M: 'static,
    F: EachInFun<M>
{
    fn run<'w>(&mut self, world: &World<'w>) {
        for (entity, 
             input) 
        in world.query::<(F::Item<'w>,InComponent<F::Channel>)>() {
            let input = In(input.get(world));

            let args = F::Params::get_arg(
                world,
            );

            self.fun.run(world, entity, input, args);
        }
    }
}    
struct IsEachIn;

impl<M, F:'static> IntoSystem<(M,IsEachIn)> for F
where
    M: 'static,
    F: EachInFun<M>
{
    type System = EachInSystem<M, F>;

    fn into_system(this: Self, world: &mut World) -> Self::System {
        EachInSystem::new(world, this)
    }
}

macro_rules! impl_each_in_params {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F:'static, C:Channel, T:Query, $($param: Param),*> 
        EachInFun<fn(T, C, $($param,)*)> for F
        where for<'w> F:FnMut(T, In<C>, $($param),*) -> () +
            FnMut(T::Item<'w>, In<C>, $(Arg<$param>),*) -> ()
        {
            type Item<'w> = T;
            type Channel = C;
            type Params = ($($param,)*);

            fn run<'b,'w>(
                &mut self, 
                _world: &World<'w>, 
                each: T::Item<'w>, 
                input: In<Self::Channel>,
                arg: Arg<($($param,)*)>
            ) {
                let ($($param,)*) = arg;
                self(each, input, $($param,)*)
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
    type Item<'w>: Query;
    type Channel: Channel;
    type Params: Param;

    fn run<'a,'w>(&mut self, 
        world: &World<'w>,
        item: <Self::Item<'w> as Query>::Item<'w>, // <'a>, 
        out: Out<Self::Channel>,
        args: Arg<Self::Params>
    );
}

pub struct EachOutSystem<M, F>
where
    F: EachOutFun<M>
{
    fun: F,
    marker: PhantomData<M>,
}

impl<M, F:'static> EachOutSystem<M, F>
where
    F: EachOutFun<M>
{
    fn new<'w>(_world: &mut World<'w>, fun: F) -> Self {
        Self {
            fun: fun,
            marker: PhantomData,
        }
    }
}

impl<M, F:'static> System for EachOutSystem<M, F>
where
    M: 'static,
    F: EachOutFun<M>
{
    fn run<'w>(&mut self, world: &World<'w>) {
        for (entity, 
             out) 
        in world.query::<(F::Item<'w>,OutComponent<F::Channel>)>() {
            let out = Out(out.get(world));

            let args = F::Params::get_arg(
                world,
            );

            self.fun.run(world, entity, out, args);
        }
    }
}    
struct IsEachOut;

impl<M, F:'static> IntoSystem<(M,IsEachOut)> for F
where
    M: 'static,
    F: EachOutFun<M>
{
    type System = EachOutSystem<M, F>;

    fn into_system(this: Self, world: &mut World) -> Self::System {
        EachOutSystem::new(world, this)
    }
}

macro_rules! impl_each_out_params {
    ($($param:ident),*) => {
        #[allow(non_snake_case)]
        impl<F:'static, C:Channel, T:Query, $($param: Param),*> 
        EachOutFun<fn(T, C, $($param,)*)> for F
        where for<'w> F:FnMut(T, Out<C>, $($param),*) -> () +
            FnMut(T::Item<'w>, Out<C>, $(Arg<$param>),*) -> ()
        {
            type Item<'w> = T;
            type Channel = C;
            type Params = ($($param,)*);

            fn run<'b,'w>(
                &mut self, 
                _world: &World<'w>, 
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
    use crate::{prelude::App, world::prelude::World, system::channel_system::Out};

    use super::{In, Channel, InChannel, InComponent, OutComponent, OutChannel};

    use std::{rc::Rc, cell::RefCell};

    use essay_ecs_macros::Component;

    #[test]
    fn each_in() {
        let mut app = App::new();

        app.spawn((TestA(1), InChannelTest::new_box()));

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(move |t :&mut TestA, mut input: In<TestChannel>| {
            ptr.borrow_mut().push(format!("{:?}", t));
            ptr.borrow_mut().push(format!("{:?}", input.read()));
        });

        app.update();
        assert_eq!(take(&values), "TestA(1), Some(\"value-b(1)\")");

        app.update();
        assert_eq!(take(&values), "TestA(1), Some(\"value-a(2)\")");

        app.update();
        assert_eq!(take(&values), "TestA(1), None");
    }

    #[test]
    fn each_out() {
        let mut app = App::new();

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        app.spawn((TestA(1), OutChannelTest::new_box(values.clone())));

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

    fn take(values: &Rc<RefCell<Vec<String>>>) -> String {
        let v : Vec<String> = values.borrow_mut().drain(..).collect();

        v.join(", ")
    }

    #[derive(Component,PartialEq, Debug)]
    struct TestA(u32);
    #[derive(Debug)]
    struct TestInFiber(usize);

    struct TestChannel;

    impl Channel for TestChannel {
        type In<'a> = InChannelTestItem<'a>;
        type Out<'a> = OutChannelTestItem<'a>;
    }

    struct InChannelTest {
        values: Vec<String>,
    }

    struct InChannelTestItem<'a> {
        fiber_in: &'a mut InChannelTest,
        tick: u64,
    }

    impl InChannelTest {
        fn new() -> Self {
            let mut values = Vec::<String>::new();
            values.push("value-a".to_string());
            values.push("value-b".to_string());

            Self {
                values: values,
            }
        }

        fn new_item(&mut self, tick: u64) -> InChannelTestItem {
            InChannelTestItem::new(self, tick)
        }

        fn new_box() -> InComponent<TestChannel> {
            Box::new(Self::new())
        }
    }

    impl InChannel for InChannelTest {
        type Channel = TestChannel;

        fn get(&mut self, world: &World) -> InChannelTestItem {
            self.new_item(u64::from(world.ticks()))
        }
    }

    impl<'a> InChannelTestItem<'a> {
        fn new(fiber_in: &'a mut InChannelTest, tick: u64) -> Self {
            Self {
                fiber_in: fiber_in,
                tick: tick,
            }
        }

        fn read(&mut self) -> Option<String> {
            match self.fiber_in.values.pop() {
                Some(value) => {
                    Some(format!("{}({:?})", value, self.tick))
                }
                None => None,
            }
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

        fn get(&mut self, world: &World) -> OutChannelTestItem {
            self.new_item(u64::from(world.ticks()))
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