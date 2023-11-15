use std::{collections::HashSet, any::{type_name, Any}};

use super::app::App;

///
/// see bevy_app/src/plugin.rs
/// 
pub trait Plugin {
    fn build(&self, app: &mut App);

    fn name(&self) -> &str {
        type_name::<Self>()
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
    plugins: Vec<Box<dyn DynPlugin>>,
    names: HashSet<String>,
}

impl Plugins {
    pub(crate) fn add_name<P: Plugin>(&mut self, plugin: &P) {
        if plugin.is_unique() && !self.names.insert(plugin.name().to_string()) {
            panic!("Attemped to add duplicate plugin {}", plugin.name());
        }
    }

    pub(crate) fn push<P: Plugin + 'static>(&mut self, plugin: P) {
        self.plugins.push(Box::new(PluginItem::new(plugin)));
    }

    pub(crate) fn contains_plugin<T:Plugin>(&self) -> bool {
        self.names.contains(type_name::<T>())
    }

    pub(crate) fn get_plugin<P: Plugin + 'static>(&self) -> Option<&P> {
        let name = type_name::<P>();

        for plugin in &self.plugins {
            if plugin.name() == name {
                let any : &dyn Any = plugin.as_any();

                return any.downcast_ref::<P>();
            }
        }

        None
    }

    pub(crate) fn get_plugin_mut<P: Plugin + 'static>(&mut self) -> Option<&mut P> {
        let name = type_name::<P>();

        for plugin in &mut self.plugins {
            if plugin.name() == name {
                let any : &mut dyn Any = plugin.as_any_mut();

                return any.downcast_mut::<P>();
            }
        }

        None
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

struct PluginItem<P: Plugin> {
    plugin: P,
}

impl<P: Plugin + 'static> PluginItem<P> {
    fn new(plugin: P) -> Self {
        Self {
            plugin
        }
    }
}

impl<P: Plugin + 'static> DynPlugin for PluginItem<P> {
    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn cleanup(&self, app: &mut App) {
        self.plugin.cleanup(app);
    }

    fn finish(&self, app: &mut App) {
        self.plugin.finish(app);
    }

    fn as_any(&self) -> &dyn Any {
        &self.plugin
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        &mut self.plugin
    }
}

trait DynPlugin {
    fn name(&self) -> &str;
    fn finish(&self, app: &mut App);
    fn cleanup(&self, app: &mut App);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
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