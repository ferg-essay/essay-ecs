use essay_ecs_core_macros::ScheduleLabel;

use crate::{
    entity::{View, ViewIterator}, 
    error::Result,
    schedule::{ScheduleLabel, SystemMeta, ExecutorFactory, UnsafeStore}, 
    Store, Schedule, IntoSystemConfig, 
    Schedules, IntoSystem, 
    system::System, store::FromStore, IntoPhaseConfigs,
};

mod ecs { pub mod core { pub use crate::*; } }
use ecs as essay_ecs;

///
/// ECS application only using the essay_ecs_core crate.
/// 
/// Primarily for testing, but it also serves as a focus to visualize 
/// the core API.
/// 
/// Applications should generally use essay_ecs::App instead. 
/// 
pub struct CoreApp {
    store: Store,
    main_schedule: Box<dyn ScheduleLabel>,
}

impl CoreApp {
    pub fn new() -> Self {
        CoreApp::default()
    }

    pub fn empty() -> Self {
        let mut store = Store::new();

        store.init_resource::<Schedules>();

        CoreApp {
            store,
            main_schedule: Box::new(Core),
        }
    }

    pub fn system<M>(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        system: impl IntoSystemConfig<M>
    ) -> &mut Self {
        self.resource_mut::<Schedules>().add_system(
            label,
            system
        );
    
        self
    }

    pub fn phase(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        into_phases: impl IntoPhaseConfigs
    ) -> &mut Self {
        self.resource_mut::<Schedules>().add_phases(
            label,
            into_phases
        );
    
        self
    }

    pub fn get_resource<T: Send + 'static>(&mut self) -> Option<&T> {
        self.store.get_resource::<T>()
    }

    pub fn get_mut_resource<T: Send + 'static>(&mut self) -> Option<&mut T> {
        self.store.get_resource_mut::<T>()
    }

    pub fn resource<T: Send + 'static>(&mut self) -> &T {
        self.store.get_resource::<T>().expect("unassigned resource")
    }

    pub fn resource_mut<T: Send + 'static>(&mut self) -> &mut T {
        self.store.get_resource_mut::<T>().expect("unassigned resource")
    }

    pub fn contains_resource<T: 'static>(&mut self) -> bool {
        self.store.contains_resource::<T>()
    }

    pub fn init_resource<T: FromStore + Send + 'static>(&mut self) -> &mut Self {
        self.store.init_resource::<T>();

        self
    }

    pub fn insert_resource<T:Send + 'static>(&mut self, value: T) {
        self.store.insert_resource(value);
    }

    pub fn query<Q:View>(&mut self) -> ViewIterator<Q> {
        self.store.query()
    }

    pub fn get_mut_schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>
    ) -> Option<&mut Schedule> {
        self.store.resource_mut::<Schedules>().get_mut(label)
    }

    pub fn run_system<M>(&mut self, into_system: impl IntoSystem<(), M>) -> Result<()> {
        let mut system = IntoSystem::into_system(into_system);
        
        let mut meta = SystemMeta::empty();
        
        let mut store = UnsafeStore::new(self.store.take());
        system.init(&mut meta, &mut store)?;
        system.run(&mut store).unwrap();
        system.flush(&mut store);

        self.store.replace(store.take());

        Ok(())
    }

    pub fn eval<O, M>(&mut self, into_system: impl IntoSystem<O, M>) -> Result<O> {
        self.store.eval(into_system)
    }

    pub fn tick(&mut self) -> Result<()> {
        self.store.run_schedule(&self.main_schedule)
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
        let schedule = Schedule::new();
        app.resource_mut::<Schedules>()
            .insert(Core, schedule);

        app
    }
}

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Hash, Eq)]
pub struct Core;

#[cfg(test)]
mod test {
    use super::{Core, CoreApp};

    #[test]
    fn test_schedule() {
        let mut app = CoreApp::new();

        app.system(Core, || println!("tick!"));

        app.tick().unwrap();

    }
}

