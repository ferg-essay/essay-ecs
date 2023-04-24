use crate::{World, entity::{EntityId, Component}};

pub struct EntityRef<'a> {
    id: EntityId,
    
    world: &'a World,
}

pub struct EntityMut<'a> {
    id: EntityId,
    
    world: &'a mut World,
}

impl<'a> EntityRef<'a> {
    pub(crate) fn new(id: EntityId, world: &'a World) -> Self {
        Self {
            id,
            world,
        }
    }

    pub fn get<T:Component>(&self) -> Option<&T> {
        self.world.get::<T>(self.id)
    }
}

impl<'a> EntityMut<'a> {
    pub(crate) fn new(id: EntityId, world: &'a mut World) -> Self {
        Self {
            id,
            world,
        }
    }

    pub fn despawn(&mut self) {
        self.world.despawn(self.id);
    }
}