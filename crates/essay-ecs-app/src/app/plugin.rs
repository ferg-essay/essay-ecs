use std::{collections::HashSet, any::type_name};

use super::app::App;

///
/// see bevy_app/src/plugin.rs
/// 
pub trait Plugin {
    fn build(&self, app: &mut App);

    fn setup(&self, _app: &mut App) {
    }

    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    fn is_unique(&self) -> bool {
        true
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

    pub(crate) fn is_plugin_added<T:Plugin>(&self) -> bool {
        self.names.contains(type_name::<T>())
    }

    pub(crate) fn drain(&mut self) -> Vec<Box<dyn Plugin>> {
        self.plugins.drain(..).collect::<Vec<Box<dyn Plugin>>>()
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

    use crate::app::app::App;

    use super::Plugin;

    #[test]
    fn add_plugin() {
        let mut app = App::new();

        assert!(! app.is_plugin_added::<TestSpawn>());

        app.add_plugin(TestSpawn::new(TestA(100)));

        assert!(app.is_plugin_added::<TestSpawn>());

        let values = Rc::new(RefCell::new(Vec::<TestA>::new()));

        let ptr = values.clone();
        app.eval(move |t: &TestA| ptr.borrow_mut().push(t.clone()));
        assert_eq!(take(&values), "TestA(100)");
    }

    #[test]
    #[should_panic]
    fn add_dup() {
        let mut app = App::new();

        app.add_plugin(TestSpawn::new(TestA(100)));
        app.add_plugin(TestSpawn::new(TestA(200)));
    }

    fn take<T:fmt::Debug>(ptr: &Rc<RefCell<Vec<T>>>) -> String {
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
            app.eval(move |mut c: Commands| c.spawn(value.clone()));
        }
    }
}