use std::marker::PhantomData;

use crate::{entity::{View, ViewPlan, ComponentId, ViewIterator}, schedule::{SystemMeta, UnsafeWorld}, World};

use super::Param;


pub struct Query<'w, 's, Q:View> {
    world: &'w UnsafeWorld,
    plan: &'s ViewPlan,
    marker: PhantomData<Q>,
}

//impl<'w, 's, Q: View> Query<'w, 's, Q> {
impl<'w, 's, Q:View> Query<'w, 's, Q> {
    fn new(world: &'w UnsafeWorld, plan: &'s ViewPlan) -> Self {
        Self {
            world,
            plan,
            marker: Default::default(),
        }
    }

    pub fn iter(&self) -> ViewIterator<Q> {
        unsafe { self.world.as_mut().view_iter_from_plan(&self.plan) }
    }
}

impl<Q:View> Param for Query<'_, '_, Q>
{
    type Arg<'w, 's> = Query<'w, 's, Q>;
    type State = ViewPlan;

    fn init(meta: &mut SystemMeta, world: &mut World) -> Self::State {
        let plan = world.view_build::<Q>();
        
        for id in plan.components() {
            meta.insert_component(ComponentId::from(*id));
        }
        
        for id in plan.mut_components() {
            meta.insert_component_mut(ComponentId::from(*id));
        }

        plan
    }

    fn arg<'w, 's>(
        world: &'w UnsafeWorld,
        state: &'s mut Self::State, 
    ) -> Self::Arg<'w, 's> {
        Query::new(world, state)
    }
}

#[cfg(test)]
mod test {
    use std::{sync::{Arc, Mutex}, time::Duration, thread};

    use crate::{core_app::{CoreApp, CoreSchedule}, entity::Component, Commands, schedule::Executors};

    use super::{Query};

    #[test]
    fn query_component() {
        let mut app = CoreApp::new();

        let values = Arc::new(Mutex::new(Vec::<String>::new()));
        let ptr = values.clone();
        app.add_system(move |q: Query<&TestA>| { 
            for t in q.iter() {
                ptr.lock().unwrap().push(format!("{:?}", t)); 
            }
        });

        app.tick();
        assert_eq!(take(&values), "");

        app.run_system(|mut c: Commands| { 
            c.spawn(TestA(10));
        });

        app.tick();
        assert_eq!(take(&values), "TestA(10)");

        app.tick();
        assert_eq!(take(&values), "TestA(10)");

        app.run_system(|mut c: Commands| { 
            c.spawn(TestA(20));
        });

        app.tick();
        assert_eq!(take(&values), "TestA(10), TestA(20)");
        
    }

    #[test]
    fn query_parallel_sequential() {
        let mut app = CoreApp::new();

        app.get_mut_schedule(&CoreSchedule::Main).unwrap().set_executor(Executors::Multithreaded);

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(move |_q: Query<&TestA>| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });

        let ptr = values.clone();
        app.add_system(move |_q: Query<&TestA>| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });
        
        let ptr = values.clone();
        app.add_system(move |_q: Query<&mut TestA>| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(move |_q: Query<&mut TestA>| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B], [C, [C, C], C]");
    }

    fn push(values: &Arc<Mutex<Vec<String>>>, value: String) {
        values.lock().unwrap().push(value);
    }

    fn take(values: &Arc<Mutex<Vec<String>>>) -> String {
        let v : Vec<String> = values.lock().unwrap().drain(..).collect();

        v.join(", ")
    }

    #[derive(Debug)]
    pub struct TestA(u32);

    impl Component for TestA {}
}
