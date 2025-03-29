use crate::{
    entity::{Bundle, Component, EntityId, EntityStore, View, ViewIterator, ViewPlan}, 
    error::Result,
    resource::{ResourceId, Resources}, 
    schedule::{ScheduleLabel, Schedules, SystemMeta, UnsafeStore}, 
    system::System,
    IntoSystem, 
    Schedule,
};

use super::{entity_ref::EntityMut, EntityRef};

pub struct Store(Option<StoreInner>);

impl Store {
    pub fn new() -> Self {
        Self(Some(StoreInner {
                entities: EntityStore::new(),
                resources: Resources::new(),
                resources_non_send: Resources::new(),
            }))
    }

    fn deref(&self) -> &StoreInner {
        self.0.as_ref().unwrap() 
    }

    fn deref_mut(&mut self) -> &mut StoreInner {
        self.0.as_mut().unwrap()
    }

    fn is_active(&self) -> bool {
        self.0.is_some()
    }

    //
    // Entities
    //

    pub fn entity(&mut self, id: EntityId) -> EntityRef {
        self.get_entity(id).unwrap_or_else(|| panic!("unknown entity {:?}", id))
    }

    pub fn entity_mut(&mut self, id: EntityId) -> EntityMut {
        self.get_entity_mut(id).unwrap_or_else(|| panic!("unknown entity {:?}", id))
    }

    pub fn get_entity(&self, id: EntityId) -> Option<EntityRef> {
        match self.deref().entities.get_entity(id) {
            Some(id) => Some(EntityRef::new(id, self)),
            None => None,
        }
    }

    pub fn get_entity_mut(&mut self, id: EntityId) -> Option<EntityMut> {
        match self.deref_mut().entities.get_entity(id) {
            Some(id) => Some(EntityMut::new(id, self)),
            None => None,
        }
    }

    pub fn get<T:'static>(&self, id: EntityId) -> Option<&T> {
        self.deref().entities.get::<T>(id)
    }

    pub fn get_mut<T:'static>(&mut self, id: EntityId) -> Option<&mut T> {
        self.deref_mut().entities.get_mut::<T>(id)
    }

    pub(crate) fn alloc_entity_id(&mut self) -> EntityId {
        self.deref_mut().entities.alloc_entity_id()
    }

    pub fn spawn<T:Bundle>(&mut self, value: T) -> EntityId {
        let id = self.alloc_entity_id();

        self.spawn_id::<T>(id, value)
    }

    pub(crate) fn spawn_id<T:Bundle>(&mut self, id: EntityId, value: T) -> EntityId {
        self.deref_mut().entities.spawn_id::<T>(id, value)
    }

    pub(crate) fn spawn_empty_id(&mut self, id: EntityId) -> EntityId {
        self.deref_mut().entities.spawn_empty_id(id)
    }

    pub(crate) fn insert<T:Component + 'static>(
        &mut self, 
        id: EntityId, 
        value: T
    ) -> EntityId {
        self.deref_mut().entities.extend(id, value)
    }

    pub(crate) fn despawn(&mut self, id: EntityId) {
        self.deref_mut().entities.despawn(id)
    }

    pub fn view<V: View>(&mut self) -> ViewIterator<'_,V> {
        self.deref_mut().entities.iter_view::<V>()
    }

    //
    // Resources
    //
    
    pub fn get_resource<T: Send + 'static>(&self) -> Option<&T> {
        self.deref().resources.get::<T>()
    }
    
    pub fn get_resource_mut<T: Send + 'static>(&mut self) -> Option<&mut T> {
        // TODO!
        self.deref_mut().resources.get_mut::<T>()
    }
    
    pub fn resource<T: Send + 'static>(&self) -> &T {
        self.get_resource::<T>().unwrap()
    }
    
    pub fn resource_mut<T: Send + 'static>(&mut self) -> &mut T {
        self.get_resource_mut::<T>().unwrap()
    }

    pub fn contains_resource<T: 'static>(&self) -> bool {
        self.deref().resources.contains_resource::<T>()
    }

    pub fn init_resource<T:FromStore + Send +'static>(&mut self) {
        if ! self.deref().resources.get::<T>().is_none() {
            return;
        }

        let value = T::from_store(self);

        self.insert_resource::<T>(value);
    }

    pub fn insert_resource<T:Send + 'static>(&mut self, value: T) {
        self.deref_mut().resources.insert::<T>(value)
    }

    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.deref_mut().resources.remove::<T>()
    }

    pub fn get_resource_id<T: 'static>(&mut self) -> ResourceId {
        self.deref_mut().resources.get_resource_id::<T>()
    }

    pub fn init_resource_non_send<T: FromStore + 'static>(&mut self) {
        if ! self.deref().resources_non_send.get::<T>().is_none() {
            return;
        }

        let value = T::from_store(self);

        self.insert_resource_non_send::<T>(value);
    }

    pub fn insert_resource_non_send<T: 'static>(&mut self, value: T) {
        self.deref_mut().resources_non_send.insert::<T>(value)
    }

    pub fn remove_resource_non_send<T: 'static>(&mut self) -> Option<T> {
        self.deref_mut().resources_non_send.remove::<T>()
    }

    pub fn query<Q:View>(&mut self) -> ViewIterator<Q> {
        self.deref_mut().entities.iter_view()
    }

    pub(crate) fn view_build<Q:View>(&mut self) -> ViewPlan {
        self.deref_mut().entities.view_plan::<Q>()
    }

    pub(crate) unsafe fn view_iter_from_plan<Q: View>(&mut self, plan: &ViewPlan) -> ViewIterator<Q> {
        self.deref_mut().entities.iter_view_with_plan::<Q>(plan.clone())
    }

    //
    // Schedules
    //

    pub fn add_schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        schedule: Schedule,
    ) {
        self.resource_mut::<Schedules>()
            .insert(label, schedule);
    }

    pub fn contains_schedule(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
    ) -> bool {
        self.resource_mut::<Schedules>()
            .contains(label)
    }

    pub fn run_schedule(&mut self, label: impl AsRef<dyn ScheduleLabel>) -> Result<()> {
        self.try_run_schedule(label)
    }

    pub fn run_schedule_optional(&mut self, label: impl AsRef<dyn ScheduleLabel>) -> Result<()> {
        let label = label.as_ref();

        if self.contains_schedule(label) {
            self.try_run_schedule(label)
        } else {
            Ok(())
        }
    }

    pub fn try_run_schedule(&mut self, label: impl AsRef<dyn ScheduleLabel>) -> Result<()> {
        self.try_eval_schedule(label, |world, schedule| {
            schedule.tick(world)
        })
    }

    pub fn try_eval_schedule<R>(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>,
        fun: impl FnOnce(&mut Store, &mut Schedule) -> Result<R>
    ) -> Result<R> {
        if ! self.is_active() {
            return Err("store is closed, possibly from a previous fatal error".into())
        }

        let label = label.as_ref();

        let Some((label, mut schedule))
            = self.get_resource_mut::<Schedules>()
                .and_then(|s| s.remove_entry(label))
        else {
            return Err(format!("{:?} is an unknown ScheduleLabel", label).into());
        };

        let value = fun(self, &mut schedule)?;

        self.resource_mut::<Schedules>().insert(label, schedule);

        Ok(value)
    }   

    pub fn eval<O, M>(&mut self, into_system: impl IntoSystem<O, M>) -> Result<O> {
        let mut system = IntoSystem::into_system(into_system);
        
        let mut meta = SystemMeta::empty();
    
        let mut store = UnsafeStore::new(self.take());
        system.init(&mut meta, &mut store)?;
        let value = system.run(&mut store);
        system.flush(&mut store);

        self.replace(store.take());

        value
    }

    pub(crate) fn take(&mut self) -> Self {
        let inner = self.0.take();

        Self(inner)
    }

    pub(crate) fn replace(&mut self, world: Store) {
        self.0 = world.0
    }
}

pub(crate) struct StoreInner {
    pub(crate) entities: EntityStore,
    pub(crate) resources: Resources,
    pub(crate) resources_non_send: Resources,
}

pub trait FromStore {
    fn from_store(store: &mut Store) -> Self;
}

impl<T: Default> FromStore for T {
    fn from_store(_store: &mut Store) -> T {
        T::default()
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::Component;

    use super::Store;

    #[test]
    fn spawn() {
        let mut world = Store::new();

        let id_a = world.spawn(TestA(1));

        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
        assert_eq!(world.get::<TestB>(id_a), None);

        let id_b = world.spawn(TestB(10000));

        assert_eq!(world.get::<TestA>(id_a), Some(&TestA(1)));
        assert_eq!(world.get::<TestB>(id_b), Some(&TestB(10000)));

        assert_eq!(world.get::<TestA>(id_b), None);
        assert_eq!(world.get::<TestB>(id_a), None);

        let id_b2 = world.spawn(TestB(100));

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
        let mut world = Store::new();

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
        let mut world = Store::new();
        
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

    #[derive(Clone, Debug, PartialEq)]
    struct TestA(u32);

    impl Component for TestA {}

    #[derive(Debug, PartialEq)]
    struct TestB(u16);

    impl Component for TestB {}
}