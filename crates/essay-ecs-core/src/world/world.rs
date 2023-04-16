use crate::{
    entity::{Store, ViewIterator, View, Insert, EntityId}, 
    schedule::{System, IntoSystem, ScheduleLabel, Schedules, SystemMeta}
};

use super::{resource::Resources, Ptr};

pub struct World {
    ptr: Ptr,
}

pub trait FromWorld : 'static {
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

    pub fn spawn<T:Insert>(&mut self, value: T) -> EntityId {
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
    
    pub fn get_resource<T:'static>(&self) -> Option<&T> {
        self.deref().resources.get::<T>()
    }
    
    pub fn get_resource_mut<T:'static>(&self) -> Option<&mut T> {
        // TODO!
        self.deref_mut().resources.get_mut::<T>()
    }
    
    pub fn resource<T:'static>(&self) -> &T {
        self.get_resource::<T>().unwrap()
    }
    
    pub fn resource_mut<T:'static>(&mut self) -> &mut T {
        self.get_resource_mut::<T>().unwrap()
    }

    pub fn init_resource<T:FromWorld>(&mut self) {
        if ! self.deref().resources.get::<T>().is_none() {
            return;
        }

        let value = T::init(self);

        self.insert_resource::<T>(value);
    }

    pub fn insert_resource<T:'static>(&mut self, value: T) {
        self.deref_mut().resources.insert::<T>(value)
    }

    //
    // eval
    //

    pub fn eval<R, M>(&mut self, fun: impl IntoSystem<R, M>) -> R
    {
        let mut system = IntoSystem::into_system(fun);

        system.init(&mut SystemMeta::empty(), self);
        let value = system.run(self);
        system.flush(self);

        value
    }

    //
    // Schedules
    //

    pub fn run_schedule(&mut self, label: impl ScheduleLabel) {
        let mut schedule = self.resource_mut::<Schedules>().remove(&label).unwrap();

        schedule.run(self);

        self.resource_mut::<Schedules>().insert(label, schedule);
    }
}

pub struct WorldInner {
    pub(crate) table: Store,
    pub(crate) resources: Resources,
}

impl<T:Default+'static> FromWorld for T {
    fn init(_world: &mut World) -> T {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use crate::{world::{Res, ResMut}, entity::Component};

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
    fn eval() {
        let mut world = World::new();
        assert_eq!(world.len(), 0);

        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let ptr = values.clone();
        world.eval(move |a: &TestA| push(&ptr, format!("{:?}", a)));
        assert_eq!(take(&values), "");

        world.spawn(TestA(1001));
        let ptr = values.clone();
        world.eval(move |a: &TestA| push(&ptr, format!("{:?}", a)));
        assert_eq!(take(&values), "TestA(1001)");

        world.spawn(TestA(2002));
        let ptr = values.clone();
        world.eval(move |a: &TestA| push(&ptr, format!("{:?}", a)));
        assert_eq!(take(&values), "TestA(1001), TestA(2002)");

        world.eval(|a: &mut TestA| a.0 += 1);
        let ptr = values.clone();
        world.eval(move |a: &TestA| push(&ptr, format!("{:?}", a)));
        assert_eq!(take(&values), "TestA(1002), TestA(2003)");
    }

    #[test]
    fn eval_out() {
        let mut world = World::new();
        assert_eq!(world.len(), 0);

        assert_eq!(world.eval(|| "result"), "result");

        world.insert_resource(TestA(1000));

        assert_eq!(world.eval(|r: Res<TestA>| format!("{:?}", r.get())), "TestA(1000)");
        assert_eq!(world.eval(|r: Res<TestA>| r.clone()), TestA(1000));

        assert_eq!(world.eval(|mut r: ResMut<TestA>| r.0 += 1), ());
        assert_eq!(world.eval(|r: Res<TestA>| r.clone()), TestA(1001));
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