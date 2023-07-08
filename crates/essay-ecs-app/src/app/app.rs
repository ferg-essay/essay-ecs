//use essay_ecs_macros::ScheduleLabel;

use std::mem;

use essay_ecs_core::{
    IntoPhaseConfigs,
    Schedule, Schedules,
    IntoSystem, IntoSystemConfig,
    schedule::{
        SystemMeta, ScheduleLabel, UnsafeWorld
    }, 
    World,
    system::System,
    entity::{Bundle, EntityId}, 
    Local,
};

use super::{plugin::{Plugins, Plugin}, CoreSchedule, CoreTaskSet};

///
/// see bevy ecs/../app.rs
/// 

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub struct Tick(u64);

pub struct App {
    world: World,
    plugins: Plugins,
}

impl Tick {
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl App {
    pub fn new() -> Self {
        App::default()
    }

    pub fn empty() -> Self {
        let mut world = World::new();

        world.init_resource::<Schedules>();

        App {
            world: world,
            plugins: Plugins::default(),
        }
    }

    pub fn add_system<M>(
        &mut self, 
        into_system: impl IntoSystemConfig<M>
    ) -> &mut Self
    {
        self.resource_mut::<Schedules>().add_system(
            &CoreSchedule::Main, 
            into_system
        );
    
        self
    }

    pub fn add_startup_system<M>(
        &mut self, 
        into_system: impl IntoSystemConfig<M>
    ) -> &mut Self
    {
        self.resource_mut::<Schedules>().add_system(
            &CoreSchedule::Startup, 
            into_system
        );
    
        self
    }

    pub fn get_resource<T:Send+'static>(&mut self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    pub fn get_mut_resource<T:Send+'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    pub fn resource<T:Send+'static>(&mut self) -> &T {
        self.world.get_resource::<T>().expect("unassigned resource")
    }

    pub fn resource_mut<T:Send+'static>(&mut self) -> &mut T {
        self.world.get_resource_mut::<T>().expect("unassigned resource")
    }

    pub fn insert_resource<T:Send+'static>(&mut self, value: T) {
        self.world.insert_resource(value);
    }

    pub fn add_default_schedule(&mut self) -> &mut Self {
        self.add_schedule(CoreSchedule::Main, CoreSchedule::main_schedule());
        self.add_schedule(CoreSchedule::Startup, CoreSchedule::startup_schedule());
        self.add_schedule(CoreSchedule::Outer, CoreSchedule::outer_schedule());

        self
    }

    pub fn add_schedule(
        &mut self, 
        label: impl ScheduleLabel, 
        schedule: Schedule
    ) -> &mut Self {
        self.resource_mut::<Schedules>().insert(label, schedule);

        self
    }

    pub fn spawn<T:Bundle>(&mut self, value: T) -> EntityId {
        self.world.spawn(value)
    }

    pub fn add_plugin<P:Plugin+'static>(&mut self, plugin: P) -> &mut Self {
        let plugin: Box<dyn Plugin> = Box::new(plugin);

        self.plugins.add_name(&plugin);
        plugin.build(self);
        self.plugins.push(plugin);

        self
    }

    pub fn is_plugin_added<P:Plugin>(&self) -> bool {
        self.plugins.is_plugin_added::<P>()
    }

    pub fn setup(&mut self) -> &mut Self {
        for plugin in self.plugins.drain() {
            plugin.setup(self);
        }

        self
    }

    pub fn update(&mut self) -> &mut Self {
        self.world.resource_mut::<Tick>().0 += 1;

        self.world.run_schedule(CoreSchedule::Outer);

        self
    }

    pub fn eval<R, M>(&mut self, fun: impl IntoSystem<R, M>) -> R
    {
        let mut schedule = Schedule::new();
        //schedule.add_system(fun);
        todo!();
        /*
        let mut system = IntoSystem::into_system(fun);

        system.init(&mut SystemMeta::empty(), &mut self.world);
        let value = system.run(&mut unsafe_world);
        system.flush(&mut self.world);

        value
        */
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();

        app.insert_resource(Tick(0));

        app.add_default_schedule();

        app
    }
}

impl CoreSchedule {
    fn main_schedule() -> Schedule {
        CoreTaskSet::main_schedule()
    }

    fn outer_schedule() -> Schedule {
        let mut schedule = Schedule::new();

        todo!();
        // schedule.add_system(Self::outer_system);

        schedule
    }

    fn outer_system(world: &mut World, mut is_startup: Local<bool>) {
        if ! *is_startup {
            *is_startup = true;
            world.run_schedule(CoreSchedule::Startup);
        }

        world.run_schedule(CoreSchedule::Main);
    }

    fn startup_schedule() -> Schedule {
        Schedule::new()
    }
}

impl CoreTaskSet {
    fn main_schedule() -> Schedule {
        let mut schedule = Schedule::new();

        schedule.set_default_phase(Self::Update);

        schedule.add_phases((
            Self::First,
            Self::PreUpdate,
            Self::Update,
            Self::PostUpdate,
            Self::Last,
        ).chained());

        schedule
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc, sync::{Mutex, Arc}};

    use essay_ecs_core::Res;

    use crate::app::app::App;

    #[test]
    fn app_system() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
        
        let ptr = Arc::clone(&value);
        app.add_system(move || ptr.lock().unwrap().push("update".to_string()));
        assert_eq!(take(&value), "");
        app.update();
        assert_eq!(take(&value), "update");
        app.update();
        app.update();
        assert_eq!(take(&value), "update, update");
    }

    #[test]
    fn startup_system() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Rc::new(RefCell::new(value));
        /*
        let ptr = Rc::clone(&value);
        app.add_startup_system(move || ptr.borrow_mut().push("startup".to_string()));

        let ptr = Rc::clone(&value);
        app.add_system(move || ptr.borrow_mut().push("update".to_string()));
        assert_eq!(take(&value), "");
        app.update();
        assert_eq!(take(&value), "startup, update");
        app.update();
        app.update();
        assert_eq!(take(&value), "update, update");
        */
    }

    #[test]
    fn app_resource() {
        let mut app = App::new();

        app.insert_resource(TestA(1));
        assert_eq!(app.resource::<TestA>(), &TestA(1));

        app.insert_resource(TestB(2));
        assert_eq!(app.resource::<TestA>(), &TestA(1));
        assert_eq!(app.resource::<TestB>(), &TestB(2));
    }

    #[test]
    fn eval() {
        let mut app = App::new();

        app.insert_resource(TestA(1));
        assert_eq!(app.eval(|r: Res<TestA>| r.clone()), TestA(1));

        app.insert_resource(TestB(2));
        assert_eq!(app.eval(|r: Res<TestA>| r.clone()), TestA(1));
        assert_eq!(app.eval(|r: Res<TestB>| r.clone()), TestB(2));
    }

    #[derive(Debug, Clone, PartialEq)]
    struct TestA(u32);

    #[derive(Debug, Clone, PartialEq)]
    struct TestB(u32);

    fn take(ptr: &Arc<Mutex<Vec<String>>>) -> String {
        ptr.lock().unwrap().drain(..).collect::<Vec<String>>().join(", ")
    }

    fn test_system() {
        println!("hello");
    }

}