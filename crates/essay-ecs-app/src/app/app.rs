///
/// see bevy ecs/../app.rs
/// 

use essay_ecs_core::{
    Schedule, Schedules,
    IntoSystemConfig,
    schedule::{
        ScheduleLabel,
    }, 
    World,
    entity::{Bundle, EntityId}, 
    world::FromWorld,
};

use super::{plugin::{Plugins, Plugin}, main_schedule::MainSchedulePlugin, Main};

pub struct App {
    world: World,
    plugins: Plugins,
    main_schedule: Box<dyn ScheduleLabel>,
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
            main_schedule: Box::new(Main),
        }
    }

    pub fn add_system<M>(
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

    pub fn get_resource<T:Send+'static>(&mut self) -> Option<&T> {
        self.world.get_resource::<T>()
    }

    pub fn get_mut_resource<T:Send+'static>(&mut self) -> Option<&mut T> {
        self.world.get_resource_mut::<T>()
    }

    pub fn resource<T:Send+'static>(&mut self) -> &T {
        self.world.get_resource::<T>().expect("unassigned resource")
    }

    pub fn resource_mut<T: Send + 'static>(&mut self) -> &mut T {
        self.world.get_resource_mut::<T>().expect("unassigned resource")
    }

    pub fn insert_resource<T: Send + 'static>(&mut self, value: T) {
        self.world.insert_resource(value);
    }

    pub fn init_resource<T: FromWorld + Send + 'static>(&mut self) -> &mut Self {
        self.world.init_resource::<T>();

        self
    }

    pub fn add_schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        schedule: Schedule
    ) -> &mut Self {
        self.world.add_schedule(label, schedule);

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
        self.world.run_schedule(&self.main_schedule);

        self
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();

        app.init_resource::<Schedules>();

        app.add_plugin(MainSchedulePlugin);

        app
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::{Mutex, Arc}};

    use crate::app::{app::{App}, Update, Startup};

    #[test]
    fn app_hello() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
        
        let ptr = Arc::clone(&value);
        app.add_system(Update, move || ptr.lock().unwrap().push("update".to_string()));
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
        let value = Arc::new(Mutex::new(value));
      
        let ptr = Arc::clone(&value);
        app.add_system(Startup, move || push(&ptr, "startup"));

        let ptr = Arc::clone(&value);
        app.add_system(Update, move || push(&ptr, "update"));
        assert_eq!(take(&value), "");
        app.update();
        assert_eq!(take(&value), "startup, update");
        app.update();
        app.update();
        assert_eq!(take(&value), "update, update");
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

    #[derive(Debug, Clone, PartialEq)]
    struct TestA(u32);

    #[derive(Debug, Clone, PartialEq)]
    struct TestB(u32);

    fn take(ptr: &Arc<Mutex<Vec<String>>>) -> String {
        ptr.lock().unwrap().drain(..).collect::<Vec<String>>().join(", ")
    }

    fn push(ptr: &Arc<Mutex<Vec<String>>>, value: &str) {
        ptr.lock().unwrap().push(value.to_string());
    }
}