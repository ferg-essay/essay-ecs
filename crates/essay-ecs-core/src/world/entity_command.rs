use crate::{entity::{Component, EntityId}, World, Commands};

use super::Command;

pub struct EntityCommands<'a, 'c> {
    commands: &'a mut Commands<'c>,
    id: EntityId,
}

impl<'a, 'c> EntityCommands<'a, 'c> {
    pub(crate) fn new(commands: &'a mut Commands<'c>, id: EntityId) -> Self {
        Self {
            commands,
            id,
        }
    }

    pub fn despawn(&mut self) {
        self.commands.add(Despawn::new(self.id));
    }
}

///
/// world.spawn()
/// 
pub(crate) struct Spawn<T:Component+'static> {
    value: T,
}

impl<T:Component + 'static> Spawn<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            value
        }
    }
}

impl<T:Component + 'static> Command for Spawn<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.spawn(self.value);
    }
}

///
/// world.despawn()
/// 
pub(crate) struct Despawn {
    id: EntityId,
}

impl Despawn {
    pub(crate) fn new(id: EntityId) -> Self {
        Self {
            id
        }
    }
}

impl Command for Despawn {
    fn flush(self: Box<Self>, world: &mut World) {
        world.despawn(self.id);
    }
}

#[cfg(test)]
mod tests {
    use crate::{entity::{Component, EntityId}, base_app::BaseApp, Commands};

    #[test]
    fn spawn() {
        let mut app = BaseApp::new();

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
    fn despawn() {
        let mut app = BaseApp::new();

        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100)]);

        let mut values: Vec<EntityId> = app.query::<(&TestA,EntityId)>()
            .map(|(_, id)| id)
            .collect();
        let id = values.pop().unwrap();
        
        app.run_system(move |mut c: Commands| { c.entity(id).despawn(); });

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![]);
    }

    #[derive(Clone, PartialEq, Debug, Default)]
    pub struct TestA(usize);

    impl Component for TestA {}
}


