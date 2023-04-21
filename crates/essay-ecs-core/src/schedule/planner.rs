use core::fmt;
use std::{collections::{HashMap, HashSet}, hash};

use crate::{world::ResourceId};

use super::{preorder::{Preorder, NodeId}, plan::Plan, system::SystemId, phase::PhaseId};


pub struct Planner {
    systems: Vec<SystemMeta>,

    preorder: Preorder,

    order: Vec<SystemId>,
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Priority(u32);

pub struct SystemMeta {
    id: SystemId,
    name: String,
    phase: Option<SystemId>,

    priority: Priority,

    is_exclusive: bool,
    is_flush: bool,

    resources: HashSet<ResourceId>,
    mut_resources: HashSet<ResourceId>,
}

pub struct SystemAccessGroup {
    phase: Option<PhaseId>,
    
    is_exclusive: bool,
    is_flush: bool, 

    resources: Vec<ResourceId>,
    mut_resources: Vec<ResourceId>,

    systems: Vec<SystemId>,
}

impl PartialEq for SystemAccessGroup {
    fn eq(&self, other: &Self) -> bool {
        self.phase == other.phase
        && self.is_exclusive == other.is_exclusive
        && self.is_flush == other.is_flush
        && self.resources == other.resources
        && self.mut_resources == other.mut_resources
    }
}

impl hash::Hash for SystemAccessGroup {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.phase.hash(state);
        self.is_exclusive.hash(state);
        self.is_flush.hash(state);
        self.resources.hash(state);
        self.mut_resources.hash(state);
    }
}

impl Planner {
    pub(crate) fn new() -> Self {
        Self {
            systems: Default::default(),
            // uninit_systems: Default::default(),
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
            preorder.add_arrow(
                NodeId::from(*prev_id),
                NodeId::from(*next_id)
            );

            map.insert(*next_id, *prev_id);
            prev_id = next_id;
        }

        map
    }

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
            order: Default::default(),
        }
    }
}

impl SystemMeta {
    pub(crate) fn new(
        id: SystemId, 
        name: String,
        phase: Option<SystemId>,
    ) -> Self {
        Self {
            id,
            name,
            phase,
            priority: Default::default(),

            is_flush: false,
            is_exclusive: false,

            resources: Default::default(),
            mut_resources: Default::default(),
        }
    }

    pub fn empty() -> Self {
        Self {
            id: SystemId(0),
            name: "empty".to_string(),
            priority: Default::default(),
            phase: None,

            is_flush: false,
            is_exclusive: false,

            resources: Default::default(),
            mut_resources: Default::default(),
        }
    }

    pub fn id(&self) -> SystemId {
        self.id
    }

    pub fn set_exclusive(&mut self) {
        self.is_exclusive = true;
    }

    pub fn is_exclusive(&self) -> bool {
        self.is_exclusive
    }

    pub fn set_flush(&mut self) {
        self.is_flush = true;
    }

    pub fn is_flush(&self) -> bool {
        self.is_flush
    }

    pub fn priority(&self) -> Priority {
        self.priority
    }

    pub fn set_priority(&mut self, priority: Priority) {
        self.priority = priority;
    }

    pub fn add_priority(&mut self, delta: u32) {
        self.priority = self.priority.add(delta);
    }

    pub fn sub_priority(&mut self, delta: u32) {
        self.priority = self.priority.sub(delta);
    }
    
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
}

impl fmt::Debug for SystemMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemMeta")
         .field("id", &self.id)
         .field("name", &self.name)
         .field("phase", &self.phase)
         .field("is_exclusive", &self.is_exclusive)
         .field("is_flush", &self.is_exclusive)
         .finish()
    }
}

impl Priority {
    pub const HIGH : Priority = Priority(2000);
    pub const DEFAULT : Priority = Priority(1000);
    pub const LOW : Priority = Priority(500);

    pub fn value(&self) -> u32 {
        self.0
    }

    pub fn add(&self, value: u32) -> Priority {
        Priority(self.0 + value)
    }

    pub fn sub(&self, value: u32) -> Priority {
        Priority(self.0 - value)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::DEFAULT
    }
}

impl From<u32> for Priority {
    fn from(value: u32) -> Self {
        Priority(value)
    }
}

#[cfg(test)]
mod test {
    use std::{sync::{Arc, Mutex}, thread, time::Duration};

    use crate::{base_app::{BaseApp, BaseSchedule}, Res, schedule::{schedule::ExecutorType}, ResMut};

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