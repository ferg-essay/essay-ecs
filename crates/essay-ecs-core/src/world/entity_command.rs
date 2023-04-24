use crate::{entity::{Component, EntityId}, World, Commands};

use super::Command;

pub struct EntityCommands<'a, 'w, 's> {
    commands: &'a mut Commands<'w, 's>,
    id: EntityId,
}

impl<'a, 'w, 's> EntityCommands<'a, 'w, 's> {
    pub(crate) fn new(commands: &'a mut Commands<'w, 's>, id: EntityId) -> Self {
        Self {
            commands,
            id,
        }
    }

    pub fn insert<T:Component + 'static>(&mut self, value: T) -> &mut Self {
        self.commands.add(EntityInsert::new(self.id, value));

        self
    }

    pub fn despawn(&mut self) {
        self.commands.add(EntityDespawn::new(self.id));
    }
}

///
/// world.spawn_empty()
/// 
pub(crate) struct SpawnEmpty {
    id: EntityId,
}

impl SpawnEmpty {
    pub(crate) fn new(id: EntityId) -> Self {
        Self {
            id,
        }
    }
}

impl Command for SpawnEmpty {
    fn flush(self: Box<Self>, world: &mut World) {
        world.spawn_empty_id(self.id);
    }
}

///
/// world.spawn()
/// 
pub(crate) struct Spawn<T:Component + 'static> {
    id: EntityId,
    value: T,
}

impl<T:Component + 'static> Spawn<T> {
    pub(crate) fn new(id: EntityId, value: T) -> Self {
        Self {
            id,
            value,
        }
    }
}

impl<T:Component + 'static> Command for Spawn<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.spawn_id(self.id, self.value);
    }
}

///
/// world.insert()
/// 
pub(crate) struct EntityInsert<T:Component> {
    id: EntityId,
    value: T,
}

impl<T:Component + 'static> EntityInsert<T> {
    pub(crate) fn new(id: EntityId, value: T) -> Self {
        Self {
            id,
            value
        }
    }
}

impl<T:Component + 'static> Command for EntityInsert<T> {
    fn flush(self: Box<Self>, world: &mut World) {
        world.insert(self.id, self.value);
    }
}

///
/// world.despawn()
/// 
pub(crate) struct EntityDespawn {
    id: EntityId,
}

impl EntityDespawn {
    pub(crate) fn new(id: EntityId) -> Self {
        Self {
            id
        }
    }
}

impl Command for EntityDespawn {
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
    fn spawn_empty_insert() {
        let mut app = BaseApp::new();

        app.run_system(|mut c: Commands| {
            c.spawn_empty().insert(TestA(100));
        });

        let values: Vec<TestA> = app.query::<&TestA>()
            .map(|t| t.clone())
            .collect();
        assert_eq!(values, vec![TestA(100)]);

        app.run_system(|mut c: Commands| {
            c.spawn_empty().insert(TestA(200)).insert(TestB(201));
        });

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

    #[derive(Clone, PartialEq, Debug, Default)]
    pub struct TestB(usize);

    impl Component for TestB {}
}


