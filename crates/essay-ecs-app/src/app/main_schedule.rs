use essay_ecs_core::{ScheduleLabel, schedule::{ScheduleLabel, Executors}, Store, Local, Schedule};

use super::{plugin::Plugin, App};

mod essay_ecs { pub mod core { pub use essay_ecs_core::*; }}


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct Main;

impl Main {
    fn main_system(world: &mut Store, mut is_init: Local<bool>) {
        if ! *is_init {
            *is_init = true;
            let _ = world.try_run_schedule(PreStartup);
            let _ = world.try_run_schedule(Startup);
            let _ = world.try_run_schedule(PostStartup);
        }

        let labels : Vec<Box<dyn ScheduleLabel>> = world
            .resource::<MainSchedule>().schedules
            .iter()
            .map(|x| x.box_clone())
            .collect();
        for label in labels {
            let _ = world.try_run_schedule(label);
        }
    }
}

pub struct MainSchedule {
    schedules: Vec<Box<dyn ScheduleLabel>>,
}

impl Default for MainSchedule {
    fn default() -> Self {
        Self { 
            schedules: vec![
                Box::new(First),
                Box::new(PreUpdate),
                Box::new(Update),
                Box::new(PostUpdate),
                Box::new(Last),
                ],
        }
    }
}

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreStartup;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Startup;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PostStartup;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct First;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PreUpdate;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Update;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PostUpdate;

#[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Last;

pub struct MainSchedulePlugin;

impl Plugin for MainSchedulePlugin {
    fn build(&self, app: &mut App) {
        let mut main_schedule = Schedule::new();
        main_schedule.set_executor(Executors::Single);

        app.schedule(Main, main_schedule)
            .init_resource::<MainSchedule>()
            .system(Main, Main::main_system);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, Arc};

    use essay_ecs_core::ScheduleLabel;

    use crate::app::{App, Update, Startup, main_schedule::{PostStartup, PreStartup, First, PreUpdate, PostUpdate, Last}};

    mod essay_ecs { pub mod core { pub use essay_ecs_core::*; }}
    
    #[test]
    fn app_hello() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
        
        let ptr = Arc::clone(&value);
        app.system(Update, move || ptr.lock().unwrap().push("update".to_string()));
        assert_eq!(take(&value), "");
        app.tick().unwrap();
        assert_eq!(take(&value), "update");
        app.tick().unwrap();
        app.tick().unwrap();
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
        app.tick().unwrap();
        assert_eq!(take(&value), "startup, update");
        app.tick().unwrap();
        app.tick().unwrap();
        assert_eq!(take(&value), "update, update");
    }

    #[test]
    fn startup_pre_post() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
      
        let ptr = Arc::clone(&value);
        app.system(Startup, move || push(&ptr, "startup"));

        let ptr = Arc::clone(&value);
        app.system(PostStartup, move || push(&ptr, "post-startup"));

        let ptr = Arc::clone(&value);
        app.system(PreStartup, move || push(&ptr, "pre-startup"));

        let ptr = Arc::clone(&value);
        app.system(Update, move || push(&ptr, "update"));
        assert_eq!(take(&value), "");

        app.tick().unwrap();
        assert_eq!(take(&value), "pre-startup, startup, post-startup, update");
        app.tick().unwrap();
        app.tick().unwrap();
        assert_eq!(take(&value), "update, update");
    }

    #[test]
    fn all_schedules() {
        let mut app = App::new();
        let value = Vec::<String>::new();
        let value = Arc::new(Mutex::new(value));
      
        let ptr = Arc::clone(&value);
        app.system(Startup, move || push(&ptr, "startup"));

        let ptr = Arc::clone(&value);
        app.system(PostStartup, move || push(&ptr, "post-startup"));

        let ptr = Arc::clone(&value);
        app.system(PreStartup, move || push(&ptr, "pre-startup"));

        let ptr = Arc::clone(&value);
        app.system(Update, move || push(&ptr, "update"));

        let ptr = Arc::clone(&value);
        app.system(First, move || push(&ptr, "first"));

        let ptr = Arc::clone(&value);
        app.system(PreUpdate, move || push(&ptr, "pre-update"));

        let ptr = Arc::clone(&value);
        app.system(PostUpdate, move || push(&ptr, "post-update"));

        let ptr = Arc::clone(&value);
        app.system(Last, move || push(&ptr, "last"));

        let ptr = Arc::clone(&value);
        app.system(Bogus, move || push(&ptr, "bogus"));

        assert_eq!(take(&value), "");
        app.tick().unwrap();
        assert_eq!(take(&value), "pre-startup, startup, post-startup, first, pre-update, update, post-update, last");
        app.tick().unwrap();
        assert_eq!(take(&value), "first, pre-update, update, post-update, last");
        app.tick().unwrap();
        assert_eq!(take(&value), "first, pre-update, update, post-update, last");
    }

    #[derive(ScheduleLabel, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct Bogus;

    fn take(ptr: &Arc<Mutex<Vec<String>>>) -> String {
        ptr.lock().unwrap().drain(..).collect::<Vec<String>>().join(", ")
    }

    fn push(ptr: &Arc<Mutex<Vec<String>>>, value: &str) {
        ptr.lock().unwrap().push(value.to_string());
    }
}