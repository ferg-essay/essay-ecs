use core::fmt;
use std::{collections::{HashMap, HashSet}, hash};

use crate::{resource::ResourceId, entity::ComponentId, system::SystemId};

use super::{preorder::{Preorder, NodeId}, plan::Plan, phase::{PhaseItem, PhaseId, PhasePreorder}, Phase};


pub struct Planner {
    phases: PhasePreorder,

    systems: Vec<SystemMeta>,

    preorder: Preorder,

    order: Vec<SystemId>,
}

impl Planner {
    pub(crate) fn new() -> Self {
        Self {
            phases: PhasePreorder::new(),
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
        phase_id: PhaseId,
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

    pub(crate) fn sort(&mut self) {
        let phase_order = self.phases.sort();

        let mut preorder = self.preorder.clone();

        /*
        let prev_map = self.prev_map(
            &mut preorder, 
            phase_order
        );
        */
        self.add_phase_arrows(&mut preorder, phase_order);

        for meta in &self.systems {
            if ! meta.is_marker() {
                self.add_system_phase_arrows(&mut preorder, meta);
            }
        }

        self.order = preorder.sort().iter()
            .map(|n| SystemId::from(*n))
            .collect();
    }

    pub(crate) fn plan(&self, phase_order: Vec<PhaseId>) -> Plan {
        let mut preorder = self.preorder.clone();

        preorder = PhasePlan::plan(self, preorder, PhaseId::zero());

        for phase in &phase_order {
            preorder = PhasePlan::plan(self, preorder, *phase);
        }
        /*
        let prev_map = self.prev_map(
            &mut preorder, 
            phase_order
        );
        */

        for meta in &self.systems {
            if ! meta.is_marker() {
                self.add_system_phase_arrows(&mut preorder, meta);
            }
        }

        Plan::new(&mut preorder)
    }

    fn add_system_phase_arrows(&self, preorder: &mut Preorder, meta: &SystemMeta) {
        let phase = &self.phases[meta.phase_id];

        preorder.add_arrow(
            NodeId::from(phase.first()),
            NodeId::from(meta.id), 
        );

        preorder.add_arrow(
            NodeId::from(meta.id), 
            NodeId::from(phase.last())
        );
    }

    fn add_phase_arrows(
        &self, 
        preorder: &mut Preorder,
        phase_order: Vec<PhaseId>
    ) {
        let mut iter = phase_order.iter();

        let Some(prev_id) = iter.next() else { return };

        let mut prev_id = prev_id;

        for phase_id in iter {
            let prev_phase = &self.phases[*prev_id];
            let next_phase = &self.phases[*phase_id];

            preorder.add_arrow(
                NodeId::from(prev_phase.last()),
                NodeId::from(next_phase.first()),
            );

            prev_id = phase_id;
        }
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

    pub(crate) fn add_phase(&mut self, phase: &Box<dyn Phase>) -> PhaseId {
        self.phases.add_box_phase(phase)
    }

    pub(crate) fn phases_mut(&mut self) -> &mut PhasePreorder {
        &mut self.phases
    }
}

impl Default for Planner {
    fn default() -> Self {
        Self { 
            phases: PhasePreorder::new(),
            systems: Default::default(), 
            preorder: Default::default(),
            order: Default::default(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Priority(u32);

pub struct SystemMeta {
    id: SystemId,
    name: String,

    phase_id: PhaseId,

    priority: Priority,

    is_exclusive: bool,
    is_marker: bool,

    resources: HashSet<ResourceId>,
    mut_resources: HashSet<ResourceId>,

    components: HashSet<ComponentId>,
    mut_components: HashSet<ComponentId>,
}

impl SystemMeta {
    pub(crate) fn new(
        id: SystemId, 
        name: String,
        phase_id: PhaseId,
    ) -> Self {
        Self {
            id,
            name,
            phase_id,
            priority: Default::default(),

            is_marker: false,
            is_exclusive: false,

            resources: Default::default(),
            mut_resources: Default::default(),

            components: Default::default(),
            mut_components: Default::default(),
        }
    }

    pub fn empty() -> Self {
        Self {
            id: SystemId(0),
            name: "empty".to_string(),
            priority: Default::default(),
            phase_id: PhaseId::zero(),

            is_marker: false,
            is_exclusive: false,

            resources: Default::default(),
            mut_resources: Default::default(),

            components: Default::default(),
            mut_components: Default::default(),
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

    pub(crate) fn set_marker(&mut self) {
        self.is_marker = true;
    }

    pub(crate) fn is_marker(&self) -> bool {
        self.is_marker
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

    pub fn insert_resource(&mut self, id: ResourceId) {
        self.resources.insert(id);
    }

    pub fn insert_resource_mut(&mut self, id: ResourceId) {
        self.mut_resources.insert(id);
    }

    pub fn insert_component(&mut self, id: ComponentId) {
        self.components.insert(id);
    }

    pub fn insert_component_mut(&mut self, id: ComponentId) {
        self.mut_components.insert(id);
    }
    
    /*
    pub(crate) fn add_phase_arrows(
        &self, 
        preorder: &mut Preorder, 
        prev_map: &HashMap<SystemId, SystemId>
    ) {
        for phase in &self.phases {
            preorder.add_arrow(
                NodeId::from(phase.first()),
                NodeId::from(self.id), 
            );

            preorder.add_arrow(
                NodeId::from(self.id), 
                NodeId::from(phase.last())
            );
        }
    }
    */

    //pub(crate) fn set_phase(&mut self, system_id: SystemId) {
    //    self.phases = Some(system_id);
    //}
}

impl fmt::Debug for SystemMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemMeta")
         .field("id", &self.id)
         .field("name", &self.name)
         // .field("phases", &self.phases)
         .field("is_exclusive", &self.is_exclusive)
         .field("is_flush", &self.is_exclusive)
         .field("resources", &self.resources)
         .field("mut_resources", &self.mut_resources)
         .finish()
    }
}

pub struct AccessGroup {
    phase_id: PhaseId,
    
    is_exclusive: bool,
    is_flush: bool, 

    resources: Vec<ResourceId>,
    mut_resources: Vec<ResourceId>,

    components: Vec<ComponentId>,
    mut_components: Vec<ComponentId>,

    systems: Vec<SystemId>,

    first: Option<SystemId>,
    last: Option<SystemId>,
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

pub struct PhasePlan {
    phase_id: PhaseId,

    group_map: HashMap<AccessGroup, AccessGroupId>,
    groups: Vec<AccessGroup>,

    exclusive: Option<AccessGroupId>,

    resource_mut_map: HashMap<ResourceId, Vec<AccessGroupId>>,
    component_mut_map: HashMap<ComponentId, Vec<AccessGroupId>>,
}

impl PhasePlan {
    fn plan(
        planner: &Planner, 
        mut preorder: Preorder,
        phase_id: PhaseId
    ) -> Preorder {
        let mut phase_plan = Self {
            phase_id,

            group_map: Default::default(),
            groups: Default::default(),

            exclusive: None,

            resource_mut_map: Default::default(),
            component_mut_map: Default::default(),
        };

        phase_plan.add_systems(&planner.systems);
        phase_plan.internal_arrows(&mut preorder);
        phase_plan.group_arrows(&mut preorder);

        preorder
    }

    fn add_systems(&mut self, metas: &Vec<SystemMeta>) {
        for meta in metas.iter().filter(|m| m.phase_id == self.phase_id) {
            let group = AccessGroup::from(meta);
            let id = AccessGroupId(self.groups.len());

            let id = *self.group_map.entry(group).or_insert(id);

            if id.0 == self.groups.len() {
                let group = AccessGroup::from(meta);

                if group.is_exclusive {
                    if ! group.is_flush {
                        if self.exclusive.is_none() {
                            self.exclusive = Some(id);
                        }
                    }
                } else {
                    for resource_id in &group.mut_resources {
                        let groups = self.resource_mut_map
                            .entry(*resource_id)
                            .or_insert_with(|| Vec::new());
                    
                        groups.push(id);
                    }

                    for component_id in &group.mut_components {
                        let groups = self.component_mut_map
                            .entry(*component_id)
                            .or_insert_with(|| Vec::new());
                    
                        groups.push(id);
                    }
                }

                self.groups.push(group);
            }

            let group = &mut self.groups[id.0];

            group.systems.push(meta.id());
        }
    }

    fn internal_arrows(&mut self, preorder: &mut Preorder) {
        for group in &mut self.groups {
            group.internal_arrows(preorder);
        }
    }

    fn group_arrows(&self, preorder: &mut Preorder) {
        for group in &self.groups {
            if group.is_exclusive {
                continue;
            }

            for id in &group.resources {
                let id = *id;

                if let Some(mut_ids) = self.resource_mut_map.get(&id) {
                    let mut_ids = mut_ids.clone();

                    self.arrows_from_tail(preorder, &mut_ids, group);
                }
            }

            for id in &group.components {
                let id = *id;

                if let Some(mut_ids) = self.component_mut_map.get(&id) {
                    let mut_ids = mut_ids.clone();

                    self.arrows_from_tail(preorder, &mut_ids, group);
                }
            }

            if let Some(exclusive) = self.exclusive {
                let exclusive_last = self.groups[exclusive.0].last.unwrap();

                match group.first {
                    Some(first) => { 
                        preorder.add_arrow(
                            NodeId::from(exclusive_last),
                            NodeId::from(first),
                        );
                    },
                    None => {
                        for id in &group.systems {
                            preorder.add_arrow(
                                NodeId::from(exclusive_last),
                                NodeId::from(*id),
                            );
                        }
                    }
                };
            }
        }
    }

    fn arrows_from_tail(
        &self, 
        preorder: &mut Preorder, 
        mut_ids: &[AccessGroupId], 
        group: &AccessGroup
    ) {
        for id in mut_ids.iter().rev() {
            let last = self.groups[id.0].last.clone();
            
            if let Some(last) = last {
                group.arrows_from_tail(preorder, last);
                return;
            }
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AccessGroupId(usize);
impl AccessGroup {
    fn is_write(&self) -> bool {
        self.is_exclusive
        || self.is_flush
        || ! self.mut_resources.is_empty()
        || ! self.mut_components.is_empty()
    }

    fn internal_arrows(&mut self, preorder: &mut Preorder) {
        if self.is_write() {
            let mut iter = self.systems.iter();

            let Some(prev_id) = iter.next() else { return };
    
            let mut prev_id = prev_id;

            self.first = Some(*prev_id);
    
            for next_id in iter {
                preorder.add_arrow(
                    NodeId::from(*prev_id),
                    NodeId::from(*next_id)
                );
                // println!("  Arrow {:?} -> {:?}", prev_id, next_id);
    
                prev_id = next_id;
            }

            self.last = Some(*prev_id);
        }
    }

    fn arrows_from_tail(&self, preorder: &mut Preorder, tail: SystemId) {
        for id in &self.systems {
            preorder.add_arrow(
                NodeId::from(tail),
                NodeId::from(*id),
            );
        }
    }
}

impl From<&SystemMeta> for AccessGroup {
    fn from(meta: &SystemMeta) -> Self {
        let mut group = AccessGroup {
            phase_id: meta.phase_id,

            is_exclusive: meta.is_exclusive, 
            is_flush: meta.is_marker,

            resources: meta.resources.iter().map(|s| *s).collect(),
            mut_resources: meta.mut_resources.iter().map(|s| *s).collect(),

            components: meta.components.iter().map(|s| *s).collect(),
            mut_components: meta.mut_components.iter().map(|s| *s).collect(),

            systems: Vec::new(),

            first: None,
            last: None,
        };

        group.resources.sort();
        group.mut_resources.sort();

        group.components.sort();
        group.mut_components.sort();

        group
    }
}

impl PartialEq for AccessGroup {
    fn eq(&self, other: &Self) -> bool {
        self.phase_id == other.phase_id
        && self.is_exclusive == other.is_exclusive
        && self.is_flush == other.is_flush
        && self.resources == other.resources
        && self.mut_resources == other.mut_resources
        && self.components == other.components
        && self.mut_components == other.mut_components
    }
}

impl Eq for AccessGroup {}

impl hash::Hash for AccessGroup {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.phase_id.hash(state);

        self.is_exclusive.hash(state);
        self.is_flush.hash(state);

        self.resources.hash(state);
        self.mut_resources.hash(state);

        self.components.hash(state);
        self.mut_components.hash(state);
    }
}

impl fmt::Debug for AccessGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessGroup")
        .field("phase", &self.phase_id)
        .field("is_exclusive", &self.is_exclusive)
        .field("is_flush", &self.is_flush)
        .field("resources", &self.resources)
        .field("mut_resources", &self.mut_resources)
        .field("components", &self.components)
        .field("mut_components", &self.mut_components)
        .field("systems", &self.systems)
        .field("first", &self.first)
        .field("last", &self.last)
        .finish()
    }
}

#[cfg(test)]
mod test {
    use std::{sync::{Arc, Mutex}, thread, time::Duration};

    use essay_ecs_macros::Phase;

    use crate::{
        core_app::{CoreApp, Core}, 
        entity::Component, 
        Res, ResMut, Commands, Store, schedule::Executors, IntoSystemConfig, IntoPhaseConfigs
    };

    mod ecs { pub mod core { pub use crate::*; }}
    use ecs as essay_ecs;

    #[test]
    fn phase_groups() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        app.add_phases(Core, (TestPhases::A, TestPhases::B, TestPhases::C).chained());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        }).phase(TestPhases::C));
        
        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        }).phase(TestPhases::C));
        
        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        }).phase(TestPhases::B));
        
        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        }).phase(TestPhases::B));

        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        }).phase(TestPhases::A));
        
        let ptr = values.clone();
        app.add_system(Core, (move || {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        }).phase(TestPhases::A));

        app.tick();

        assert_eq!(take(&values), "[A, [A, A], A], [B, [B, B], B], [C, [C, C], C]");
    }

    #[test]
    fn world_mut_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_w: &mut Store| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_w: &mut Store| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move || {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move || {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B], [C, [C, C], C]");
    }

    #[test]
    fn res_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |res: Res<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |res: Res<String>| {
            push(&ptr, format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S-{}]", res.get()));
        });

        app.tick();

        assert_eq!(take(&values), "[S-test, [S-test, S-test], S-test]");
        
    }

    #[test]
    fn resmut_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |res: ResMut<String>| {
            push(&ptr, format!("[A-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A-{}]", res.get()));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |res: ResMut<String>| {
            push(&ptr, format!("[B-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B-{}]", res.get()));
        });

        app.tick();

        assert_eq!(take(&values), "[A-test, A-test], [B-test, B-test]");
    }

    #[test]
    fn resmut_disjoint_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());
        app.insert_resource(10 as u32);

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |res: ResMut<String>| {
            push(&ptr, format!("[A-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A-{}]", res.get()));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |res: ResMut<u32>| {
            thread::sleep(Duration::from_millis(10));
            push(&ptr, format!("[B-{}", res.get()));
            thread::sleep(Duration::from_millis(50));
            push(&ptr, format!("B-{}]", res.get()));
        });

        app.tick();

        assert_eq!(take(&values), "[A-test, [B-10, B-10], A-test]");
    }

    #[test]
    fn res_resmut_parallel_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_res: Res<String>| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });

        let ptr = values.clone();
        app.add_system(Core, move |_res: Res<String>| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_res: ResMut<String>| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_res: ResMut<String>| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B], [C, [C, C], C]");
    }

    #[test]
    fn comp_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_item: &TestA| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_item: &TestA| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, [A, A], A]");
        
    }

    #[test]
    fn comp_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestA| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestA| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B]");
        
    }

    #[test]
    fn comp_mut_disjoint() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));
        app.run_system(|mut c: Commands| c.spawn(TestB(200)));

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestA| {
            push(&ptr, format!("[S"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestB| {
            push(&ptr, format!("[S"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("S]"));
        });

        app.tick();

        assert_eq!(take(&values), "[S, S], [S, S]");
        
    }

    #[test]
    fn comp_sequential_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestA| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_item: &mut TestA| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_item: &TestA| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });

        let ptr = values.clone();
        app.add_system(Core, move |_item: &TestA| {
            push(&ptr, format!("[C"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("C]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B], [C, [C, C], C]");
        
    }

    #[test]
    fn resmut_cycle() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());
        app.insert_resource(10 as u32);

        let values = Arc::new(Mutex::new(Vec::<String>::new()));

        let ptr = values.clone();
        app.add_system(Core, move |_r1: Res<u32>, _r2: ResMut<String>| {
            push(&ptr, format!("[A"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("A]"));
        });
        
        let ptr = values.clone();
        app.add_system(Core, move |_r1: ResMut<u32>, _r2: Res<String>| {
            push(&ptr, format!("[B"));
            thread::sleep(Duration::from_millis(100));
            push(&ptr, format!("B]"));
        });

        app.tick();

        assert_eq!(take(&values), "[A, A], [B, B]");
    }

    struct TestA(u32);
    struct TestB(u32);

    impl Component for TestA {}
    impl Component for TestB {}

    fn push(values: &Arc<Mutex<Vec<String>>>, value: String) {
        values.lock().unwrap().push(value);
    }

    fn take(values: &Arc<Mutex<Vec<String>>>) -> String {
        let values : Vec<String> = values.lock().unwrap().drain(..).collect();

        values.join(", ")
    }

    #[derive(Phase, Clone, Copy, Debug, PartialEq, Eq, Hash)]
    enum TestPhases {
        A,
        B,
        C,
    }
}