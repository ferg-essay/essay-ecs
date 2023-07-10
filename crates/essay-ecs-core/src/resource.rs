use std::{collections::HashMap, any::{TypeId, type_name}, ptr::NonNull, alloc::Layout, mem::{ManuallyDrop, self}};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResourceId(usize);

pub struct Resources {
    resource_map: HashMap<TypeId,ResourceId>,
    resources: Vec<Option<Resource>>,
}

impl ResourceId {
    fn new(index: usize) -> Self {
        ResourceId(index)
    }

    pub fn index(&self) -> usize {
        self.0
    }
}

impl Resources {
    pub fn new() -> Self {
        Self {
            resource_map: HashMap::new(),
            resources: Vec::new(),
        }
    }

    pub fn insert<T:Send + 'static>(&mut self, value: T) {
        let id = ResourceId::new(self.resources.len());
        let type_id = TypeId::of::<T>();

        let id = *self.resource_map.entry(type_id).or_insert(id);

        if id.index() == self.resources.len() {
            self.resources.push(Some(Resource::new(id, value)));
        } else {
            // TODO: drop
            self.resources[id.index()] = Some(Resource::new(id, value));
        }
    }

    pub(crate) fn get_resource_id<T:'static>(&self) -> ResourceId {
        let type_id = TypeId::of::<T>();

        *self.resource_map.get(&type_id).unwrap()
    }

    pub fn get<T:Send + 'static>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();

        let id = self.resource_map.get(&type_id)?;

        unsafe { 
            match &self.resources[id.index()] {
                Some(resource) => Some(resource.deref()),
                None => None,
            }
        }
    }

    pub fn get_mut<T: Send + 'static>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();

        let id = self.resource_map.get(&type_id)?;

        unsafe { 
            match &self.resources[id.index()] {
                Some(resource) => Some(resource.deref_mut()),
                None => None,
            }
        }
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        let type_id = TypeId::of::<T>();

        let id = self.resource_map.get(&type_id)?;

        unsafe { 
            match self.resources[id.index()].take() {
                Some(resource) => Some(resource.take()),
                None => None,
            }
        }
    }

    pub fn _insert_non_send<T:'static>(&mut self, value: T) {
        let id = ResourceId::new(self.resources.len());
        let type_id = TypeId::of::<T>();

        let id = *self.resource_map.entry(type_id).or_insert(id);

        if id.index() == self.resources.len() {
            self.resources.push(Some(Resource::new(id, value)));
        } else {
            // TODO: drop
            self.resources[id.index()] = Some(Resource::new(id, value));
        }
    }
}

struct Resource {
    _id: ResourceId,
    _name: String,
    //value: Ptr,
    data: NonNull<u8>,
}

impl Resource {
    pub(crate) fn new<T>(id: ResourceId, value: T) -> Self {
        let layout = Layout::new::<T>();
        let data = unsafe { std::alloc::alloc(layout) };
        let data = NonNull::new(data).unwrap();

        let mut resource = Self {
            _id: id,
            _name: type_name::<Self>().to_string(),
            data: data,

            // marker: Default::default(),
        };

        unsafe {
            resource.write(value);
        }

        resource
    }

    unsafe fn write<T>(&mut self, value: T) {
        let mut value = ManuallyDrop::new(value);
        let source: NonNull<u8> = NonNull::from(&mut *value).cast();

        let src = source.as_ptr();
        // TODO: drop prev
        let dst = self.data.as_ptr();

        let count = mem::size_of::<T>();

        std::ptr::copy_nonoverlapping::<u8>(src, dst, count);
    }

    pub unsafe fn deref<T>(&self) -> &T {
        &*self.data.as_ptr().cast::<T>()
    }
    
    pub unsafe fn deref_mut<T>(&self) -> &mut T {
        &mut *self.data.as_ptr().cast::<T>()
    }
    
    unsafe fn take<T>(self) -> T {
        self.data.as_ptr().cast::<T>().read()
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

    #[test]
    fn remove() {
        let mut resources = Resources::new();

        resources.insert(TestA(1));
        assert_eq!(resources.get::<TestA>(), Some(&TestA(1)));

        assert_eq!(resources.remove::<TestA>(), Some(TestA(1)));
        assert_eq!(resources.remove::<TestA>(), None);
        assert_eq!(resources.get::<TestA>(), None);
    }

    #[derive(PartialEq, Debug)]
    struct TestA(u32);

    #[derive(PartialEq, Debug)]
    struct TestB(u32);
}
