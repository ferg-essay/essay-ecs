use std::{collections::VecDeque, marker::PhantomData};

use crate::{prelude::{Param}, entity::Component, schedule::SystemMeta};

use super::{World, FromWorld};

pub trait Command: Send + Sync + 'static {
    fn flush(self: Box<Self>, world: &mut World);
}

pub struct Commands<'a> {
    queue: &'a mut CommandQueue,
}

type BoxCommand = Box<dyn Command>;

pub struct CommandQueue {
    queue: VecDeque<BoxCommand>,
}

//
// Commands/Queue Implementation
//

impl<'a> Commands<'a> {
    pub fn add(&mut self, command: impl Command + 'static) {
        self.queue.add(command);
    }
}

impl Param for Commands<'_> {
    type Arg<'w, 's> = Commands<'s>;
    type State = CommandQueue;

    fn init(_meta: &mut SystemMeta, _world: &mut World) -> Self::State {
        CommandQueue::default()
    }

    fn arg<'w,'s>(
        _world: &'w World,
        queue: &'s mut Self::State, 
    ) -> Self::Arg<'w, 's> {
        Commands {
            queue,
        }
    }

    fn flush(world: &mut World, queue: &mut Self::State) {
        queue.flush(world);
    }
}

impl CommandQueue {
    pub fn add(&mut self, command: impl Command + 'static) {
        self.queue.push_back(Box::new(command))
    }

    fn flush(&mut self, world: &mut World) {
        for command in self.queue.drain(..) {
            command.flush(world);
        }
    }
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self { queue: Default::default() }
    }
}

//
// builtin commands
//

///
/// Closure as Command. 
/// 
impl<F> Command for F
    where F: FnOnce(&mut World) + Send + Sync + 'static
{
    fn flush(self: Box<Self>, world: &mut World) {
        self(world);
    }
}

///
/// world.spawn()
/// 
struct Spawn<T:Component+'static> {
    value: T,
}

impl<T:Component+Send+Sync+'static> Command for Spawn<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.spawn(self.value);
    }
}

impl Commands<'_> {
    ///
    /// Spawn an entity
    ///
    pub fn spawn<T:Component+Send+Sync+'static>(&mut self, value: T) {
        self.add(Spawn { value: value });
    }
}

///
/// world.init_resource()
/// 
struct InitResource<T:FromWorld> {
    marker: PhantomData<T>,
}

impl<T:FromWorld> InitResource<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
        
    }
}
/*
impl<T:FromWorld> Command for InitResource<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.init_resource::<T>();
    }
}

impl Commands<'_> {
    ///
    /// init a resource
    ///
    pub fn init_resource<T:FromWorld>(&mut self) {
        self.add(InitResource::<T>::new());
    }
}
*/

///
/// world.insert_resource()
/// 
struct InsertResource<T:'static> {
    value: T,
}

impl<T:Send+Sync+'static> Command for InsertResource<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.insert_resource(self.value);
    }
}

impl Commands<'_> {
    ///
    /// insert a resource value, overwriting any old value.
    ///
    pub fn insert_resource<T:Send+Sync+'static>(&mut self, value: T) {
        self.add(InsertResource { value });
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;
    use std::{rc::Rc, cell::RefCell};

    use crate::{world::{World, Res, ResMut}, entity::Component, Schedule};

    use super::Commands;

    #[test]
    fn add_closure() {
        /*
        let mut world = World::new();

        let values = Rc::new(RefCell::new(Vec::<TestA>::new()));

        world.eval(|mut c: Commands| c.add(|w: &mut World| {
            w.spawn(TestA(100)); 
        }));

        let ptr = values.clone();
        world.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100)");

        world.eval(|mut c: Commands| c.add(|w: &mut World| {
            w.spawn(TestA(200)); 
        }));

        let ptr = values.clone();
        world.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100), TestA(200)");
        */
    }

    #[test]
    fn spawn() {
        /*
        let mut world = World::new();

        let values = Rc::new(RefCell::new(Vec::<TestA>::new()));

        world.eval(|mut c: Commands| c.spawn(TestA(100)));

        let ptr = values.clone();
        world.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100)");

        world.eval(|mut c: Commands| c.spawn(TestA(200)));

        let ptr = values.clone();
        world.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100), TestA(200)");
        */
    }

    #[test]
    fn init_resource() {
        let mut world = World::new();
        /*
        world.eval(|mut c: Commands| c.init_resource::<TestA>());
        assert_eq!(world.eval(|r: Res<TestA>| r.clone()), TestA(0));

        world.eval(|mut r: ResMut<TestA>| r.0 += 100);
        assert_eq!(world.eval(|r: Res<TestA>| r.clone()), TestA(100));

        world.eval(|mut c: Commands| c.init_resource::<TestA>());
        assert_eq!(world.eval(|r: Res<TestA>| r.clone()), TestA(100));
        */
    }

    #[test]
    fn insert_resource() {
        let mut world = World::new();

        let mut schedule = Schedule::new();
        schedule.add_system(|mut c: Commands| c.insert_resource(TestA(100)));
        schedule.run(&mut world);

        assert_eq!(world.resource::<TestA>(), &TestA(100));

        let mut schedule = Schedule::new();
        schedule.add_system(|mut c: Commands| c.insert_resource(TestA(1000)));
        schedule.run(&mut world);

        assert_eq!(world.resource::<TestA>(), &TestA(1000));
    }

    #[derive(Clone, PartialEq, Debug, Default)]
    pub struct TestA(usize);

    impl Component for TestA {}

    fn take<T:fmt::Debug>(queue: &Rc<RefCell<Vec<T>>>) -> String {
        let values : Vec<String> = queue.borrow_mut().drain(..)
            .map(|v| format!("{:?}", v))
            .collect();

        values.join(", ")
    }
}


