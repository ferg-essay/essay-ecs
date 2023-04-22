use std::mem;

use crate::{
    entity::{Store, ViewIterator, View, Bundle, EntityId, ViewPlan}, 
    schedule::{ScheduleLabel, Schedules, SystemMeta}, prelude::Param
};

use super::{resource::Resources, Ptr, ResourceId};

pub struct World {
    ptr: Ptr,
}

pub trait FromWorld {
    fn init(world: &mut World) -> Self;
}

impl World {
    pub fn new() -> Self {
        Self {
            ptr: Ptr::new(WorldInner {
                table: Store::new(),
                resources: Resources::new(),
            }),
        }
    }

    fn deref(&self) -> &WorldInner {
        unsafe { self.ptr.deref::<WorldInner>() }
    }

    fn deref_mut(&self) -> &mut WorldInner {
        unsafe { self.ptr.deref_mut::<WorldInner>() }
    }

    pub fn len(&self) -> usize {
        self.deref().table.len()
    }

    pub fn spawn<T:Bundle>(&mut self, value: T) -> EntityId {
        self.deref_mut().table.spawn::<T>(value)
    }

    pub fn get<T:'static>(&mut self, id: EntityId) -> Option<&T> {
        self.deref_mut().table.get::<T>(id)
    }

    pub fn get_mut<T:'static>(&mut self, id: EntityId) -> Option<&mut T> {
        self.deref_mut().table.get_mut::<T>(id)
    }

    pub fn view<V:View>(&self) -> ViewIterator<'_,V> {
        self.deref_mut().table.iter_view::<V>()
    }

    //
    // Resources
    //
    
    pub fn get_resource<T:Send + 'static>(&self) -> Option<&T> {
        self.deref().resources.get::<T>()
    }
    
    pub fn get_resource_mut<T:Send + 'static>(&self) -> Option<&mut T> {
        // TODO!
        self.deref_mut().resources.get_mut::<T>()
    }
    
    pub fn resource<T:Send + 'static>(&self) -> &T {
        self.get_resource::<T>().unwrap()
    }
    
    pub fn resource_mut<T:Send + 'static>(&mut self) -> &mut T {
        self.get_resource_mut::<T>().unwrap()
    }

    pub fn init_resource<T:FromWorld + Send +'static>(&mut self) {
        if ! self.deref().resources.get::<T>().is_none() {
            return;
        }

        let value = T::init(self);

        self.insert_resource::<T>(value);
    }

    pub fn insert_resource<T:Send + 'static>(&mut self, value: T) {
        self.deref_mut().resources.insert::<T>(value)
    }

    pub fn get_resource_id<T:'static>(&mut self) -> ResourceId {
        self.deref_mut().resources.get_resource_id::<T>()
    }

    pub fn query<Q:View>(&mut self) -> ViewIterator<Q> {
        self.deref_mut().table.iter_view()
    }

    pub(crate) fn view_build<Q:View>(&self) -> ViewPlan {
        self.deref_mut().table.view_plan::<Q>()
    }

    pub(crate) unsafe fn view_iter_from_plan<Q: View>(&self, plan: &ViewPlan) -> ViewIterator<Q> {
        self.deref_mut().table.iter_view_with_plan::<Q>(plan.clone())
    }

    //
    // Schedules
    //

    pub fn run_schedule(&mut self, label: impl ScheduleLabel) {
        let mut schedule = self.resource_mut::<Schedules>().remove(&label).unwrap();

        schedule.tick(self).unwrap();

        self.resource_mut::<Schedules>().insert(label, schedule);
    }

    pub(crate) fn take(&mut self) -> Self {
        let ptr = mem::replace(&mut self.ptr, Ptr::new(WorldInner {
            table: Store::new(),
            resources: Resources::new(),
        }));

        Self {
            ptr,
        }
    }

    pub(crate) fn replace(&mut self, world: World) {
        self.ptr = world.ptr
    }
}

impl Param for &World {
    type Arg<'w, 's> = &'w World;
    type State = ();

    fn arg<'w, 's>(
        world: &'w World,
        _state: &'s mut Self::State,
    ) -> Self::Arg<'w, 's> {
        world
    }

    fn init(_meta: &mut SystemMeta, _world: &mut World) -> Self::State {
    }
}

pub struct WorldInner {
    pub(crate) table: Store,
    pub(crate) resources: Resources,
}

impl<T:Default> FromWorld for T {
    fn init(_world: &mut World) -> T {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use crate::{params::{Res, ResMut}, entity::Component};

    use super::World;

    #[test]
    fn spawn() {
        let mut world = World::new();
        assert_eq!(world.len(), 0);

        let id_a = world.spawn(TestA(1));
        assert_eq!(world.len(), 1);

        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
        assert_eq!(world.get::<TestB>(id_a), None);

        let id_b = world.spawn(TestB(10000));
        assert_eq!(world.len(), 2);

        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
        assert_eq!(world.get::<TestB>(id_b), Some(&TestB(10000)));

        assert_eq!(world.get::<TestA>(id_b), None);
        assert_eq!(world.get::<TestB>(id_a), None);

        let id_b2 = world.spawn(TestB(100));
        assert_eq!(world.len(), 3);

        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
        assert_eq!(world.get::<TestA>(id_b), None);
        assert_eq!(world.get::<TestA>(id_b2), None);

        assert_eq!(world.get::<TestB>(id_b), Some(&TestB(10000)));
        assert_eq!(world.get::<TestB>(id_b2), Some(&TestB(100)));

        world.get_mut::<TestB>(id_b).unwrap().0 += 1;
        world.get_mut::<TestB>(id_b2).unwrap().0 += 1;

        assert_eq!(world.get::<TestB>(id_b), Some(&TestB(10001)));
        assert_eq!(world.get::<TestB>(id_b2), Some(&TestB(101)));
        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
    }

    #[test]
    fn resource_set_get() {
        let mut world = World::new();

        assert_eq!(world.get_resource::<TestB>(), None);
        assert_eq!(world.get_resource_mut::<TestB>(), None);

        world.insert_resource(TestA(1));
        assert_eq!(world.get_resource::<TestA>(), Some(&TestA(1)));
        assert_eq!(world.get_resource_mut::<TestA>(), Some(&mut TestA(1)));
        assert_eq!(world.get_resource::<TestB>(), None);
        assert_eq!(world.get_resource_mut::<TestB>(), None);

        world.get_resource_mut::<TestA>().unwrap().0 += 1;

        assert_eq!(world.get_resource::<TestA>(), Some(&TestA(2)));
        assert_eq!(world.get_resource_mut::<TestA>(), Some(&mut TestA(2)));
        assert_eq!(world.get_resource::<TestB>(), None);
        assert_eq!(world.get_resource_mut::<TestB>(), None);

        world.insert_resource(TestA(1000));
        assert_eq!(world.get_resource::<TestA>(), Some(&TestA(1000)));
        assert_eq!(world.get_resource_mut::<TestA>(), Some(&mut TestA(1000)));
        assert_eq!(world.get_resource::<TestB>(), None);
        assert_eq!(world.get_resource_mut::<TestB>(), None);

        world.insert_resource(TestB(1001));
        assert_eq!(world.get_resource::<TestA>(), Some(&TestA(1000)));
        assert_eq!(world.get_resource_mut::<TestA>(), Some(&mut TestA(1000)));
        assert_eq!(world.get_resource::<TestB>(), Some(&TestB(1001)));
        assert_eq!(world.get_resource_mut::<TestB>(), Some(&mut TestB(1001)));
    }

    #[test]
    fn query() {
        let mut world = World::new();
        assert_eq!(world.len(), 0);

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        
        assert_eq!(world.query::<&TestA>()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<String>>()
            .join(", "),
            "");

        
        world.spawn(TestA(1001));
        
        assert_eq!(world.query::<&TestA>()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<String>>()
            .join(", "),
            "TestA(1001)");

        world.spawn(TestA(2002));

        assert_eq!(world.query::<&TestA>()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<String>>()
            .join(", "),
            "TestA(1001), TestA(2002)");

        for test in world.query::<&mut TestA>() {
            test.0 += 1;
        }

        assert_eq!(world.query::<&TestA>()
            .map(|v| format!("{:?}", v))
            .collect::<Vec<String>>()
            .join(", "),
            "TestA(1002), TestA(2003)");
    }

    fn push(ptr: &Rc<RefCell<Vec<String>>>, value: String) {
        ptr.borrow_mut().push(value);
    }

    fn take(ptr: &Rc<RefCell<Vec<String>>>) -> String {
        ptr.borrow_mut().drain(..).collect::<Vec<String>>().join(", ")
    }

    #[derive(Clone, Debug, PartialEq)]
    struct TestA(u32);

    impl Component for TestA {}

    #[derive(Debug, PartialEq)]
    struct TestB(u16);

    impl Component for TestB {}
}