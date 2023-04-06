use std::{cell::RefCell, rc::Rc};

use crate::store::{prelude::{Table, RowRef, EntityMutIterator, EntityTable, EntityRef, Entity2MutIterator}, ptr::PtrCell};

use super::resource::Resources;

pub struct World<'w> {
    ptr: PtrCell<'w,WorldInner<'w>>,
}

impl<'w> World<'w> {
    pub fn new() -> Self {
        Self {
            ptr: PtrCell::new(WorldInner::new()),
        }
    }

    pub fn add_entity<T:'static>(&mut self, value: T) -> EntityRef<T> {
        self.ptr.deref_mut().entities.push::<T>(value)
    }

    pub fn len(&self) -> usize {
        self.ptr.deref().entities.len()
    }

    pub fn iter_mut<T:'static>(&self) -> Entity2MutIterator<T> {
        self.ptr.deref_mut().entities.iter_mut_by_type::<T>()
    }

    pub fn eval<T:'static,F>(&self, fun: &mut F)
        where F: FnMut(&mut T)
    {
        for entity in self.ptr.deref_mut().entities.iter_mut_by_type::<T>() {
            fun(entity);
        }
    }

    pub fn add_resource<T:'static>(&mut self, value: T) {
        self.ptr.deref_mut().resources.set(value);
    }
    
    pub fn get_resource<T:'static>(&self) -> Option<&T> {
        self.ptr.deref_mut().resources.get_by_type()
    }
    
    pub fn get_resource_mut<T:'static>(&self) -> Option<&mut T> {
        self.ptr.deref_mut().resources.get_mut_by_type()
    }
}

pub struct WorldInner<'w> {
    entities: EntityTable<'w>,
    resources: Resources<'w>,
}

impl<'w> WorldInner<'w> {
    fn new() -> Self {
        Self {
            entities: EntityTable::new(),
            resources: Resources::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::World;

    #[test]
    fn spawn() {
        let mut world = World::new();
        assert_eq!(world.len(), 0);

        world.add_entity(TestA(1));
        assert_eq!(world.len(), 1);

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestA| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestA(1)");

        world.add_entity(TestB(10000));
        assert_eq!(world.len(), 2);

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestA| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestA(1)");

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestB| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestB(10000)");

        world.add_entity(TestB(100));
        assert_eq!(world.len(), 3);

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestA| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestA(1)");

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestB| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestB(10000),TestB(100)");

        let mut values = Vec::<String>::new();
        world.eval(&mut|t: &mut TestB| t.0 += 1);
        world.eval(&mut|t: &mut TestB| (&mut values).push(format!("{:?}", t)));
        assert_eq!(values.join(","), "TestB(10001),TestB(101)");
    }

    #[derive(Debug)]
    struct TestA(u32);

    #[derive(Debug)]
    struct TestB(u16);
}