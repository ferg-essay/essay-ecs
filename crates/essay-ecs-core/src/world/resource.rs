use std::{collections::HashMap, any::TypeId, cell::UnsafeCell};

use crate::{entity::{Store}};

use super::{cell::Ptr, World, world::FromWorld};

struct IsResource;

#[derive(Copy, Clone, Debug, PartialEq, Hash, PartialOrd)]
pub struct ResourceId(usize);

struct Resource {
    id: ResourceId,
    value: Ptr,
}

pub struct Resources {
    resource_map: HashMap<TypeId,ResourceId>,
    resources: Vec<Resource>,
    store: Store,
}

impl ResourceId {
    fn new(index: usize) -> Self {
        ResourceId(index)
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

impl Resource {
    fn new<T>(id: ResourceId, value: T) -> Self {
        Resource {
            id: id,
            value: Ptr::new(value),
        }
    }

    unsafe fn deref<T>(&self) -> &T {
        self.value.deref()
    }

    unsafe fn deref_mut<T>(&self) -> &mut T {
        self.value.deref_mut()
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resource_map: HashMap::new(),
            resources: Vec::new(),
            store: Store::new(),
            //resources: Vec::new(),
        }
    }
    /*
    pub(crate) fn init<T: FromWorld>(
        &mut self, 
        fun: impl FnOnce() -> <T as FromWorld>::Item
    ) {
        let id = ResourceId::new(self.resources.len());
        let type_id = TypeId::of::<T>();

        let id = *self.resource_map.entry(type_id).or_insert(id);

        if id.index() == self.resources.len() {
            self.resources.push(Resource::new(id, fun()));
        }
    }
    */

    pub fn insert<T:'static>(&mut self, value: T) {
        let id = ResourceId::new(self.resources.len());
        let type_id = TypeId::of::<T>();

        let id = *self.resource_map.entry(type_id).or_insert(id);

        if id.index() == self.resources.len() {
            self.resources.push(Resource::new(id, value));
        } else {
            // TODO: drop
            self.resources[id.index()] = Resource::new(id, value);
        }
    }

    pub fn get<T:'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        let id = self.resource_map.get(&type_id)?;

        unsafe { Some(self.resources[id.index()].deref()) }
    }

    pub fn get_mut<T:'static>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        let id = self.resource_map.get(&type_id)?;

        unsafe { Some(self.resources[id.index()].deref_mut()) }
    }
}

#[cfg(test)]
mod tests {
    use super::Resources;

    #[test]
    fn set_get() {
        let mut resources = Resources::new();

        assert_eq!(resources.get::<TestB>(), None);
        assert_eq!(resources.get_mut::<TestB>(), None);

        resources.insert(TestA(1));
        assert_eq!(resources.get::<TestA>(), Some(&TestA(1)));
        assert_eq!(resources.get_mut::<TestA>(), Some(&mut TestA(1)));
        assert_eq!(resources.get::<TestB>(), None);
        assert_eq!(resources.get_mut::<TestB>(), None);

        resources.get_mut::<TestA>().unwrap().0 += 1;

        assert_eq!(resources.get::<TestA>(), Some(&TestA(2)));
        assert_eq!(resources.get_mut::<TestA>(), Some(&mut TestA(2)));
        assert_eq!(resources.get::<TestB>(), None);
        assert_eq!(resources.get_mut::<TestB>(), None);

        resources.insert(TestA(1000));
        assert_eq!(resources.get::<TestA>(), Some(&TestA(1000)));
        assert_eq!(resources.get_mut::<TestA>(), Some(&mut TestA(1000)));
        assert_eq!(resources.get::<TestB>(), None);
        assert_eq!(resources.get_mut::<TestB>(), None);

        resources.insert(TestB(1001));
        assert_eq!(resources.get::<TestA>(), Some(&TestA(1000)));
        assert_eq!(resources.get_mut::<TestA>(), Some(&mut TestA(1000)));
        assert_eq!(resources.get::<TestB>(), Some(&TestB(1000)));
        assert_eq!(resources.get_mut::<TestB>(), Some(&mut TestB(1000)));
    }

    #[derive(PartialEq, Debug)]
    struct TestA(u32);

    #[derive(PartialEq, Debug)]
    struct TestB(u32);
}
