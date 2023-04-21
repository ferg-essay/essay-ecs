use std::collections::HashMap;

use crate::{schedule::SystemMeta};

use super::{preorder::{Preorder, NodeId}, plan::Plan, system::SystemId};


pub struct Planner {
    systems: Vec<SystemMeta>,
    uninit_systems: Vec<SystemId>,

    preorder: Preorder,

    order: Vec<SystemId>,
}

pub(crate) struct SystemItem {
    pub(crate) id: SystemId,
    pub(crate) meta: SystemMeta,

    //pub(crate) system: BoxedSystem,
    pub(crate) phase: Option<SystemId>,
}

impl Planner {
    pub(crate) fn new() -> Self {
        Self {
            systems: Default::default(),
            uninit_systems: Default::default(),
            preorder: Preorder::new(),
            order: Default::default(),
        }
    }
    
    pub(crate) fn add(
        &mut self, 
        id: SystemId,
        type_name: String,
        phase_id: Option<SystemId>,
    ) -> SystemId {
        // let system: BoxedSystem = Box::new(IntoSystem::into_system(system));

        let node_id = self.preorder.add_node(0);
        assert_eq!(id.index(), node_id.index());

        let id = SystemId::from(id);

        self.systems.push(SystemMeta::new(
            id, 
            type_name,
            phase_id,
        ));

        // self.uninit_systems.push(id);

        id
    }

    pub(crate) fn sort(&mut self, phase_order: Vec<SystemId>) {
        let mut preorder = self.preorder.clone();

        let prev_map = self.prev_map(
            &mut preorder, 
            phase_order
        );

        for meta in &self.systems {
            if ! meta.is_flush() {
                meta.add_phase_arrows(&mut preorder, &prev_map);
            }
        }

        self.order = preorder.sort().iter()
            .map(|n| SystemId::from(*n))
            .collect();
    }

    pub(crate) fn plan(&self, phase_order: Vec<SystemId>) -> Plan {
        let mut preorder = self.preorder.clone();

        let prev_map = self.prev_map(
            &mut preorder, 
            phase_order
        );

        for meta in &self.systems {
            if ! meta.is_flush() {
                meta.add_phase_arrows(&mut preorder, &prev_map);
            }
        }

        Plan::new(&preorder)
    }

    fn prev_map(
        &self, 
        preorder: &mut Preorder,
        task_set_order: Vec<SystemId>
    ) -> HashMap<SystemId,SystemId> {
        let mut map = HashMap::new();

        let mut iter = task_set_order.iter();

        let Some(prev_id) = iter.next() else { return map };

        let mut prev_id = prev_id;

        for next_id in iter {
            // println!("Phase set {:?} -> {:?}", prev_id, next_id);
            preorder.add_arrow(
                NodeId::from(*prev_id),
                NodeId::from(*next_id)
            );

            map.insert(*next_id, *prev_id);
            prev_id = next_id;
        }

        map
    }

    /*
    pub(crate) fn run(&mut self, world: &mut World) {
        for id in &self.order {
            let system = &mut self.systems[id.index()];
            
            if system.meta.is_flush() {
                // self.flush(world);
            } else {
                system.system.get_mut().run(world);
            }
        }
    }
    */

    pub(crate) fn meta(&self, id: SystemId) -> &SystemMeta {
        &self.systems[id.index()]
    }

    pub(crate) fn meta_mut(&mut self, id: SystemId) -> &mut SystemMeta {
        &mut self.systems[id.index()]
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self { 
            systems: Default::default(), 
            preorder: Default::default(),
            uninit_systems: Default::default(),
            order: Default::default(),
        }
    }
}

impl SystemItem {
    pub(crate) fn add_phase_arrows(
        &self, 
        preorder: &mut Preorder, 
        prev_map: &HashMap<SystemId, SystemId>
    ) {
        if let Some(phase) = &self.phase {
            preorder.add_arrow(
                NodeId::from(self.id), 
                NodeId::from(*phase)
            );

            if let Some(prev) = prev_map.get(&phase) {
                preorder.add_arrow(
                    NodeId::from(*prev), 
                    NodeId::from(self.id)
                );
            }
        }
    }
    /*
    pub(crate) unsafe fn run_unsafe(&self, world: &World) {
        self.system.as_mut().run_unsafe(world);
    }

    pub(crate) unsafe fn run(&self, world: &mut World) {
        self.system.as_mut().run(world);
    }

    pub(crate) fn system(&self) -> &BoxedSystem {
        &self.system
    }
    */
}

#[cfg(test)]
mod test {
    use std::{sync::{Arc, Mutex}, thread, time::Duration};

    use crate::{base_app::{BaseApp, BaseSchedule}, Res, schedule::{multithreaded::MultithreadedExecutor, schedule::ExecutorType}, ResMut};

    #[test]
    fn two_resource_parallel() {
        let mut app = BaseApp::new();

        app.get_mut_schedule(&BaseSchedule::Main).unwrap().set_executor(ExecutorType::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(move |res: Res<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });
        
        let ptr = values.clone();
        app.add_system(move |res: Res<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });

        app.tick();

        assert_eq!(take(&values), "[S-test, [S-test, S-test], S-test]");
        
    }


    #[test]
    fn two_resource_sequential() {
        let mut app = BaseApp::new();

        app.get_mut_schedule(&BaseSchedule::Main).unwrap().set_executor(ExecutorType::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(move |res: ResMut<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });
        
        let ptr = values.clone();
        app.add_system(move |res: ResMut<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });

        app.tick();

        assert_eq!(take(&values), "[S-test, S-test], [S-test, S-test]");
        
    }

    fn push(values: &Arc<Mutex<Vec<String>>>, value: String) {
        values.lock().unwrap().push(value);
    }

    fn take(values: &Arc<Mutex<Vec<String>>>) -> String {
        let values : Vec<String> = values.lock().unwrap().drain(..).collect();

        values.join(", ")
    }
}