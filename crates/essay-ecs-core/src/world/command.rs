use std::{collections::VecDeque};

use crate::entity::EntityId;
use crate::{entity::Component};

use crate::world::{World};

use super::entity_command::{Spawn, EntityCommands, SpawnEmpty};

pub trait Command: Send + 'static {
    fn flush(self: Box<Self>, world: &mut World);
}

pub struct Commands<'w, 's> {
    world: &'w mut World,
    queue: &'s mut CommandQueue,
}

type BoxCommand = Box<dyn Command>;

pub struct CommandQueue {
    queue: VecDeque<BoxCommand>,
}

unsafe impl Sync for CommandQueue {}

impl<'w, 's> Commands<'w, 's> {
    pub fn add(&mut self, command: impl Command + 'static) {
        self.queue.add(command);
    }

    pub(crate) fn new(
        world: &'w mut World,
        queue: &'s mut CommandQueue
    ) -> Self {
        Self {
            world,
            queue,
        }
    }
}

impl<'w, 's> Commands<'w, 's> {
    ///
    /// Reference to an entity
    ///
    pub fn entity<'a>(&'a mut self, id: EntityId) -> EntityCommands<'a, 'w, 's> {
        EntityCommands::new(self, id)
    }

    pub fn spawn_empty<'a>(&'a mut self) -> EntityCommands<'a, 'w, 's> {
        let id = self.world.alloc_entity_id();

        self.add(SpawnEmpty::new(id));

        EntityCommands::new(self, id)
    }

    ///
    /// Spawn an entity
    ///
    pub fn spawn<T:Component+'static>(&mut self, value: T) {
        let id = self.world.alloc_entity_id();

        self.add(Spawn::new(id, value));
    }
}

//
// Commands/Queue Implementation
//

impl CommandQueue {
    pub fn add(&mut self, command: impl Command + 'static) {
        self.queue.push_back(Box::new(command))
    }

    pub(crate) fn flush(&mut self, world: &mut World) {
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

//
// builtin commands
//

///
/// Entities
///

///
/// world.init_resource()
/// 
/*
struct InitResource<T:FromWorld + Send> {
    marker: PhantomData<T>,
}

impl<T:FromWorld + Send> InitResource<T> {
    fn new() -> Self {
        Self {
            marker: PhantomData,
        }
        
    }
}
*/

/*
impl<T:FromWorld + Send> Command for InitResource<T> {
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

impl Commands<'_, '_> {
    ///
    /// insert a resource value, overwriting any old value.
    ///
    pub fn insert_resource<T:Send+Sync+'static>(&mut self, value: T) {
        self.add(InsertResource { value });
    }
}

#[cfg(test)]
mod tests {
    use crate::{world::World, entity::Component, Schedule, core_app::CoreApp};

    use super::Commands;

    #[test]
    fn add_closure() {
        let mut app = CoreApp::new();
        
        app.run_system(|mut c: Commands| c.add(|w: &mut World| {
            w.spawn(TestA(100)); 
        }));

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100)]);

        app.run_system(|mut c: Commands| c.add(|w: &mut World| {
            w.spawn(TestA(200)); 
        }));

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100), TestA(200)]);
    }

    #[test]
    fn spawn() {
        let mut app = CoreApp::new();

        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100)]);

        app.run_system(|mut c: Commands| c.spawn(TestA(200)));

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100), TestA(200)]);
    }

    #[test]
    fn init_resource() {
        /*
        let mut world = World::new();
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
        schedule.tick(&mut world).unwrap();

        assert_eq!(world.resource::<TestA>(), &TestA(100));

        let mut schedule = Schedule::new();
        schedule.add_system(|mut c: Commands| c.insert_resource(TestA(1000)));
        schedule.tick(&mut world).unwrap();

        assert_eq!(world.resource::<TestA>(), &TestA(1000));
    }

    #[derive(Clone, PartialEq, Debug, Default)]
    pub struct TestA(usize);

    impl Component for TestA {}
}


