use std::{collections::HashSet, any::type_name};

use super::app::App;

///
/// see bevy_app/src/plugin.rs
/// 
pub trait Plugin {
    fn build(&self, app: &mut App);

    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn is_unique(&self) -> bool {
        true
    }

    fn finish(&self, _app: &mut App) {
    }

    fn cleanup(&self, _app: &mut App) {
    }
}

pub(crate) struct Plugins {
    plugins: Vec<Box<dyn Plugin>>,
    names: HashSet<String>,
}

impl Plugins {
    pub(crate) fn add_name(&mut self, plugin: &Box<dyn Plugin>) {
        if plugin.is_unique() && !self.names.insert(plugin.name().to_string()) {
            panic!("Attemped to add duplicate plugin {}", plugin.name());
        }
    }

    pub(crate) fn push(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }

    pub(crate) fn contains_plugin<T:Plugin>(&self) -> bool {
        self.names.contains(type_name::<T>())
    }

    pub(crate) fn finish(&self, app: &mut App) {
        for plugin in &self.plugins {
            plugin.finish(app);
        }
    }

    pub(crate) fn cleanup(&self, app: &mut App) {
        for plugin in &self.plugins {
            plugin.cleanup(app);
        }
    }
}

impl Default for Plugins {
    fn default() -> Self {
        Self { 
            plugins: Default::default(), 
            names: Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use core::fmt;
    use std::{rc::Rc, cell::RefCell};

    use essay_ecs_core::{Component, Commands};

    // simulate normal deployment
    mod ecs {
        pub mod core { pub use essay_ecs_core::*; }
    }
    use ecs as essay_ecs;

    use crate::app::{app::App, Startup};

    use super::Plugin;

    #[test]
    fn add_plugin() {
        let mut app = App::new();

        assert!(! app.contains_plugin::<TestSpawn>());

        app.plugin(TestSpawn::new(TestA(100)));
        /*
        assert!(app.is_plugin_added::<TestSpawn>());

        let values = Rc::new(RefCell::new(Vec::<TestA>::new()));

        let ptr = values.clone();
        app.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100)");
        */
    }

    #[test]
    #[should_panic]
    fn add_dup() {
        let mut app = App::new();

        app.plugin(TestSpawn::new(TestA(100)));
        app.plugin(TestSpawn::new(TestA(200)));
    }

    fn _take<T:fmt::Debug>(ptr: &Rc<RefCell<Vec<T>>>) -> String {
        let values : Vec<String> = ptr.borrow_mut()
            .drain(..)
            .map(|v| format!("{:?}", v))
            .collect();

        values.join(", ")
    }

    #[derive(Component, Clone, PartialEq, Debug)]
    struct TestA(usize);

    struct TestSpawn {
        value: TestA,
    }

    impl TestSpawn {
        fn new(value: TestA) -> Self {
            Self {
                value
            }
        }
    }

    impl Plugin for TestSpawn {
        fn build(&self, app: &mut App) {
            let value = self.value.clone();
            app.system(Startup, move |mut c: Commands| c.spawn(value.clone()));
        }
    }
}