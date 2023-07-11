use std::any::type_name;

///
/// see bevy bevy_app/../app.rs
/// 

use essay_ecs_core::{
    Schedule, Schedules,
    IntoSystemConfig,
    schedule::ScheduleLabel,
    World,
    world::FromWorld,
};

use crate::{event::{Event, Events}, First};

use super::{plugin::{Plugins, Plugin}, main_schedule::MainSchedulePlugin, Main};

#[cfg(test)]
use essay_ecs_core::entity::{Bundle, EntityId};

pub struct App {
    world: World,
    plugins: Plugins,
    main_schedule: Box<dyn ScheduleLabel>,
    runner: Box<dyn FnOnce(App) + Send>,
}

impl App {
    pub fn new() -> Self {
        App::default()
    }

    ///
    /// Minimal app without even the main schedule.
    /// 
    pub fn empty() -> Self {
        let mut world = World::new();

        world.init_resource::<Schedules>();

        App {
            world: world,
            plugins: Plugins::default(),
            main_schedule: Box::new(Main),
            runner: Box::new(run_once),
        }
    }

    pub fn system<M>(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        into_system: impl IntoSystemConfig<M>
    ) -> &mut Self {
        let schedules = self.resource_mut::<Schedules>();

        if let Some(schedule) = schedules.get_mut(label.as_ref()) {
            schedule.add_system(into_system);
        } else {
            let mut schedule = Schedule::new();
            schedule.add_system(into_system);
            schedules.insert(label, schedule);
        }
    
        self
    }

    //
    // resources
    //

    pub fn insert_resource<T: Send + 'static>(&mut self, value: T) {
        self.world.insert_resource(value);
    }

    pub fn init_resource<T: FromWorld + Send + 'static>(&mut self) -> &mut Self {
        self.world.init_resource::<T>();

        self
    }

    pub fn get_resource<T:Send+'static>(&mut self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    pub fn get_mut_resource<T:Send+'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    pub fn resource<T: Send + 'static>(&mut self) -> &T {
        match self.world.get_resource::<T>() {
            Some(value) => value,
            None => panic!("unassigned resource {:?}", type_name::<T>()),
        }
    }

    pub fn resource_mut<T: Send + 'static>(&mut self) -> &mut T {
        match self.world.get_resource_mut::<T>() {
            Some(value) => value,
            None => panic!("unassigned resource {:?}", type_name::<T>()),
        }
    }

    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.world.remove_resource()
    }

    pub fn insert_resource_non_send<T: 'static>(&mut self, value: T) {
        self.world.insert_resource_non_send(value);
    }

    pub fn init_resource_non_send<T: FromWorld + 'static>(&mut self) -> &mut Self {
        self.world.init_resource_non_send::<T>();

        self
    }

    pub fn remove_resource_non_send<T: 'static>(&mut self) -> Option<T> {
        self.world.remove_resource_non_send()
    }

    //
    // events
    //

    pub fn event<E: Event>(&mut self) -> &mut Self {
        if ! self.world.contains_resource::<Events<E>>() {
            self.init_resource::<Events<E>>()
                .system(First, Events::<E>::update);
        }

        self
    }

    //
    // plugins
    //

    pub fn plugin<P: Plugin + 'static>(&mut self, plugin: P) -> &mut Self {
        let plugin: Box<dyn Plugin> = Box::new(plugin);

        self.plugins.add_name(&plugin);
        plugin.build(self);
        self.plugins.push(plugin);

        self
    }

    pub fn contains_plugin<P:Plugin>(&self) -> bool {
        self.plugins.contains_plugin::<P>()
    }

    pub fn setup(&mut self) -> &mut Self {
        self
    }

    pub fn finish(&mut self) -> &mut Self {
        let plugins = std::mem::take(&mut self.plugins);

        plugins.finish(self);

        self.plugins = plugins;

        self
    }

    pub fn cleanup(&mut self) -> &mut Self {
        let plugins = std::mem::take(&mut self.plugins);

        plugins.cleanup(self);

        self.plugins = plugins;


        self
    }

    //
    // schedule/update routines
    //

    pub fn schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        schedule: Schedule
    ) -> &mut Self {
        self.world.add_schedule(label, schedule);

        self
    }

    pub fn tick(&mut self) -> &mut Self {
        self.world.run_schedule(&self.main_schedule);

        self
    }

    pub fn runner(&mut self, runner: impl FnOnce(App) + 'static + Send) -> &mut Self {
        self.runner = Box::new(runner);

        self
    }

    pub fn run(&mut self) {
        let mut app = std::mem::replace(self, App::empty());

        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));

        runner(app);
    }

    #[cfg(test)]
    pub fn spawn<T: Bundle>(&mut self, value: T) -> EntityId {
        self.world.spawn(value)
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();

        app.init_resource::<Schedules>();

        app.plugin(MainSchedulePlugin);

        app
    }
}

fn run_once(mut app: App) {
    app.finish();
    app.cleanup();

    app.tick();
}


#[cfg(test)]
mod tests {
    use std::sync::{Mutex, Arc};

    use essay_ecs_core::{Component, Commands};

    use crate::{app::{app::App, Update, Startup}, event::{Event, OutEvent, InEvent}, PreUpdate};

    mod ecs { pub mod core { pub use essay_ecs_core::*; }}
    use ecs as essay_ecs;

    #[test]
    fn app_hello() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
        
        let ptr = Arc::clone(&value);
        app.system(Update, move || ptr.lock().unwrap().push("update".to_string()));
        assert_eq!(take(&value), "");
        app.tick();
        assert_eq!(take(&value), "update");
        app.tick();
        app.tick();
        assert_eq!(take(&value), "update, update");
    }

    #[test]
    fn startup_system() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
      
        let ptr = Arc::clone(&value);
        app.system(Startup, move || push(&ptr, "startup"));

        let ptr = Arc::clone(&value);
        app.system(Update, move || push(&ptr, "update"));
        assert_eq!(take(&value), "");
        app.tick();
        assert_eq!(take(&value), "startup, update");
        app.tick();
        app.tick();
        assert_eq!(take(&value), "update, update");
    }

    #[test]
    fn default_run_once() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));

        let ptr = Arc::clone(&value);
        app.system(Startup, move || push(&ptr, "startup"));
      
        let ptr = Arc::clone(&value);
        app.system(Update, move || push(&ptr, "update"));
        assert_eq!(take(&value), "");

        app.run();
        assert_eq!(take(&value), "startup, update");
    }

    #[test]
    fn run_3() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));

        let ptr = Arc::clone(&value);
        app.system(Startup, move || push(&ptr, "startup"));
      
        let ptr = Arc::clone(&value);
        app.system(Update, move || push(&ptr, "update"));
        assert_eq!(take(&value), "");

        app.runner(|mut app| {
            for _ in 0..3 {
                app.tick();
            }
        });

        app.run();
        assert_eq!(take(&value), "startup, update, update, update");
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
    fn spawn_and_execute() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));

        let ptr = Arc::clone(&value);
        app.system(Startup, move |mut cmd: Commands| {
            push(&ptr, "spawn");
            cmd.spawn(CompA);
        });
      
        let ptr = Arc::clone(&value);
        app.system(Update, move |_comp: &CompA| push(&ptr, "update"));
        assert_eq!(take(&value), "");

        app.tick();
        assert_eq!(take(&value), "spawn, update");
    }

    #[test]
    fn events() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));

        app.event::<TestEvent>();

        let mut counter = 1;
        app.system(PreUpdate, move |mut writer: OutEvent<TestEvent>| {
            writer.send(TestEvent(counter));
            counter += 1;
        });

        let ptr = Arc::clone(&value);
        app.system(PreUpdate, move |mut reader: InEvent<TestEvent>| {
            for event in reader.iter() {
                push(&ptr, &format!("{:?}", event));
            }
        });

        app.tick();
        assert_eq!(take(&value), "TestEvent(1)");
        app.tick();
        assert_eq!(take(&value), "TestEvent(2)");
        app.tick();
        assert_eq!(take(&value), "TestEvent(3)");
    }

    #[derive(Component)]
    struct CompA;

    #[derive(Debug, Clone, PartialEq)]
    struct TestA(u32);

    #[derive(Debug, Clone, PartialEq)]
    struct TestB(u32);

    #[derive(Debug)]
    struct TestEvent(u32);

    impl Event for TestEvent {}

    fn take(ptr: &Arc<Mutex<Vec<String>>>) -> String {
        ptr.lock().unwrap().drain(..).collect::<Vec<String>>().join(", ")
    }

    fn push(ptr: &Arc<Mutex<Vec<String>>>, value: &str) {
        ptr.lock().unwrap().push(value.to_string());
    }
}