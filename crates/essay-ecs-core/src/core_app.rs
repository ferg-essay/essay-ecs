use essay_ecs_macros::ScheduleLabel;

use crate::{
    World, Schedule, IntoSystemConfig, 
    schedule::{ScheduleLabel, SystemMeta, ExecutorFactory, UnsafeWorld}, 
    entity::{View, ViewIterator}, 
    Schedules, IntoSystem, 
    system::System,
};

pub struct CoreApp {
    world: World,
    main_schedule: Box<dyn ScheduleLabel>,
}

pub use crate as essay_ecs_core;

impl CoreApp {
    pub fn new() -> Self {
        CoreApp::default()
    }

    pub fn empty() -> Self {
        let mut world = World::new();

        world.init_resource::<Schedules>();

        CoreApp {
            world: world,
            main_schedule: Box::new(Core),
        }
    }

    pub fn add_system<M>(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        into_system: impl IntoSystemConfig<M>
    ) -> &mut Self {
        self.resource_mut::<Schedules>().add_system(
            label,
            into_system
        );
    
        self
    }

    pub fn get_resource<T:Send + 'static>(&mut self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    pub fn get_mut_resource<T:Send + 'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    pub fn resource<T:Send + 'static>(&mut self) -> &T {
        self.world.get_resource::<T>().expect("unassigned resource")
    }

    pub fn resource_mut<T:Send + 'static>(&mut self) -> &mut T {
        self.world.get_resource_mut::<T>().expect("unassigned resource")
    }

    pub fn insert_resource<T:Send + 'static>(&mut self, value: T) {
        self.world.insert_resource(value);
    }

    pub fn query<Q:View>(&mut self) -> ViewIterator<Q> {
        self.world.query()
    }

    pub fn get_mut_schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>
    ) -> Option<&mut Schedule> {
        self.world.resource_mut::<Schedules>().get_mut(label)
    }

    pub fn run_system<M>(&mut self, into_system: impl IntoSystem<(),M>) -> &mut Self {
        let mut system = IntoSystem::into_system(into_system);
        
        let mut meta = SystemMeta::empty();
        let mut world = UnsafeWorld::new(self.world.take());
        system.init(&mut meta, &mut world);
        system.run(&mut world);
        system.flush(&mut world);

        self.world.replace(world.take());

        self
    }

    pub fn tick(&mut self) -> &mut Self {
        self.world.run_schedule(&self.main_schedule);

        self
    }

    pub fn set_executor(&mut self, executor: impl ExecutorFactory + 'static) -> &mut Self {
        self.resource_mut::<Schedules>().set_executor(executor);
        
        self
    }
}

impl Default for CoreApp {
    fn default() -> Self {
        let mut app = CoreApp::empty();

        app.insert_resource(Schedules::default());
        let mut schedule = Schedule::new();
        app.resource_mut::<Schedules>()
            .insert(Core, schedule);

        app
    }
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Hash, Eq)]
pub struct Core;
