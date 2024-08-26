use core::fmt;
use std::{collections::{HashMap, HashSet}, hash};

use crate::{resource::ResourceId, entity::ComponentId, system::SystemId};

use super::{preorder::{Preorder, NodeId}, plan::Plan, phase::{PhaseId, PhasePreorder}, Phase};


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
        let mut preorder = self.create_preorder();

        self.order = preorder.sort().iter()
            .map(|n| SystemId::from(*n))
            .collect();
    }

    pub(crate) fn plan(&mut self) -> Plan {
        // TODO: use order from sort instead of regenerating?
        Plan::new(&mut self.create_preorder())
    }

    fn create_preorder(&mut self) -> Preorder {
        let mut preorder = self.preorder.clone();

        preorder = PhasePlan::plan(self, preorder);

        for meta in &self.systems {
            if ! meta.is_marker() {
                self.add_system_phase_arrows(&mut preorder, meta);
            }
        }

        for phase_id in self.phases_mut().sort() {
            self.add_phase_arrows(&mut preorder, phase_id);
        };

        preorder
    }

    ///
    /// Add arrows from the phase head to the system and from the system to
    /// the phase tail
    /// 
    /// phase.head -> system -> phase.tail
    /// 
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
        phase_id: PhaseId
    ) {
        let target_id = self.phases[phase_id].first();

        for system_id in self.phases.incoming_systems(phase_id) {
            preorder.add_arrow(
                NodeId::from(system_id),
                NodeId::from(target_id),
            )
        }
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

pub struct PhasePlan {
    group_map: HashMap<AccessGroup, AccessGroupId>,
    groups: Vec<AccessGroup>,

    exclusive: Option<AccessGroupId>,

    resource_mut_map: HashMap<ResourceId, Vec<AccessGroupId>>,
    component_mut_map: HashMap<ComponentId, Vec<AccessGroupId>>,
}

impl PhasePlan {
    fn plan(
        planner: &Planner, 
        mut preorder: Preorder
    ) -> Preorder {
        let mut phase_plan = Self {
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

    ///
    /// Adds the systems to the plan.
    /// Systems are grouped into compatible AccessGroups.
    /// 
    fn add_systems(&mut self, metas: &Vec<SystemMeta>) {
        for meta in metas.iter() {
            let id = AccessGroupId(self.groups.len());
            let mut group = AccessGroup::from(meta);
            group.id = id;

            let id = *self.group_map.entry(group).or_insert(id);

            if id.0 == self.groups.len() { // adding new AccessGroup
                let mut group = AccessGroup::from(meta);
                group.id = id;

                if group.is_marker {
                    // markers aren't grouped
                } else if group.is_exclusive {
                    if self.exclusive.is_none() {
                        self.exclusive = Some(id);
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

    ///
    /// Adds arrows internal to the access group.
    /// Specifically, ordering writers
    /// 
    fn internal_arrows(&mut self, preorder: &mut Preorder) {
        for group in &mut self.groups {
            group.internal_arrows(preorder);
        }
    }

    ///
    /// Add arrows between groups
    /// world -> all
    /// write -> read for both resources and components
    /// 
    fn group_arrows(&self, preorder: &mut Preorder) {
        for group in &self.groups {
            if group.is_exclusive {
                continue;
            }

            // write -> read for resources
            for id in &group.resources {
                let id = *id;

                if let Some(mut_ids) = self.resource_mut_map.get(&id) {
                    let mut_ids = mut_ids.clone();

                    self.arrows_from_tail(preorder, &mut_ids, group);
                }
            }

            // write -> write for resources
            for id in &group.mut_resources {
                let id = *id;

                if let Some(mut_ids) = self.resource_mut_map.get(&id) {
                    let mut_ids = mut_ids.iter()
                        .filter(|id| id.i() < group.id.i())
                        .map(|id| *id)
                        .collect::<Vec<AccessGroupId>>();

                    self.arrows_from_tail(preorder, &mut_ids, group);
                }
            }

            // write -> read for components
            for id in &group.components {
                let id = *id;

                if let Some(mut_ids) = self.component_mut_map.get(&id) {
                    let mut_ids = mut_ids.clone();

                    self.arrows_from_tail(preorder, &mut_ids, group);
                }
            }

            // world -> all
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

    ///
    /// Adds arrows from the tail of a write group.
    /// Since write groups are internally ordered, only the arrow from 
    /// the last system is needed
    /// 
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

///
/// Systems with compatible phases, resources, and components are grouped into
/// an AccessGroup.
/// 
pub struct AccessGroup {
    id: AccessGroupId,

    phase_id: PhaseId,
    
    is_exclusive: bool,
    is_marker: bool, 

    resources: Vec<ResourceId>,
    mut_resources: Vec<ResourceId>,

    components: Vec<ComponentId>,
    mut_components: Vec<ComponentId>,

    systems: Vec<SystemId>,

    first: Option<SystemId>,
    last: Option<SystemId>,
}

impl AccessGroup {
    fn is_write(&self) -> bool {
        self.is_exclusive
        || ! self.mut_resources.is_empty()
        || ! self.mut_components.is_empty()
    }

    ///
    /// Adds arrows internal to the group, specifically writers are
    /// are ordered
    /// 
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
            id: AccessGroupId(usize::MAX),
            phase_id: meta.phase_id,

            is_exclusive: meta.is_exclusive, 
            is_marker: meta.is_marker,

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
        && self.is_marker == other.is_marker
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
        self.is_marker.hash(state);

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
        .field("is_flush", &self.is_marker)
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct AccessGroupId(usize);

impl AccessGroupId {
    fn i(&self) -> usize {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq)]
pub struct Priority(u32);

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
    use std::{thread, time::Duration};

    use crate::{
        core_app::{CoreApp, Core}, 
        entity::Component, 
        Res, ResMut, Commands, Store, schedule::Executors, util::test::TestValues
    };

    #[test]
    fn world_mut_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_w: &mut Store| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_w: &mut Store| {
            ptr.push(format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move || {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move || {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, A], [B, B], [C, [C, C], C]");
    }

    ///
    /// systems with Res<MyResource> can execute in parallel
    /// 
    #[test]
    fn resource_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |res: Res<String>| {
            ptr.push(format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("S-{}]", res.get()));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |res: Res<String>| {
            ptr.push(format!("[S-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("S-{}]", res.get()));
        });

        app.tick();

        assert_eq!(values.take(), "[S-test, [S-test, S-test], S-test]");
        
    }

    ///
    /// Systems with ResMut<MyResource> must execute sequentially
    /// 
    #[test]
    fn resmut_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |res: ResMut<String>| {
            ptr.push(format!("[A-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A-{}]", res.get()));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |res: ResMut<String>| {
            ptr.push(format!("[B-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B-{}]", res.get()));
        });

        app.tick();

        assert_eq!(values.take(), "[A-test, A-test], [B-test, B-test]");
    }

    ///
    /// Systems with disjoint ResMut<A> and ResMut<B> can execute in parallel
    /// 
    #[test]
    fn resmut_disjoint_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());
        app.insert_resource(10 as u32);

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |res: ResMut<String>| {
            ptr.push(format!("[A-{}", res.get()));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A-{}]", res.get()));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |res: ResMut<u32>| {
            thread::sleep(Duration::from_millis(10));
            ptr.push(format!("[B-{}", res.get()));
            thread::sleep(Duration::from_millis(50));
            ptr.push(format!("B-{}]", res.get()));
        });

        app.tick();

        assert_eq!(values.take(), "[A-test, [B-10, B-10], A-test]");
    }

    ///
    /// Systems with disjoint ResMut<A> and ResMut<B> can execute in parallel
    /// 
    #[test]
    fn resmut_overlap_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource(ResA);
        app.insert_resource(ResB);
        app.insert_resource(ResC);

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_a: ResMut<ResA>, _b: ResMut<ResB>| {
            ptr.push("[AB");
            thread::sleep(Duration::from_millis(100));
            ptr.push("AB]");
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_a: ResMut<ResA>, _c: ResMut<ResC>| {
            thread::sleep(Duration::from_millis(10));
            ptr.push("[BC");
            thread::sleep(Duration::from_millis(50));
            ptr.push("BC]");
        });

        app.tick();

        assert_eq!(values.take(), "[AB, AB], [BC, BC]");
    }

    ///
    /// ResMut<A> execute sequentially before parallel Res<A>
    /// 
    #[test]
    fn res_resmut_parallel_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_res: Res<String>| {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });

        let mut ptr = values.clone();
        app.system(Core, move |_res: Res<String>| {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_res: ResMut<String>| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_res: ResMut<String>| {
            ptr.push(format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, A], [B, B], [C, [C, C], C]");
    }

    ///
    /// Component read &TestA can execute in parallel
    /// 
    #[test]
    fn comp_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_item: &TestA| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_item: &TestA| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, [A, A], A]");
        
    }

    ///
    /// mutable components &mut TestA must execute sequentially
    /// 
    #[test]
    fn comp_sequential() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestA| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestA| {
            ptr.push(format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, A], [B, B]");
        
    }

    ///
    /// Disjoint components &mut A and &mut B can execute in parallel
    /// 
    #[test]
    fn comp_mut_disjoint() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));
        app.run_system(|mut c: Commands| c.spawn(TestB(200)));

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestA| {
            ptr.push(format!("[S"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("S]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestB| {
            ptr.push(format!("[S"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("S]"));
        });

        app.tick();

        assert_eq!(values.take(), "[S, S], [S, S]");
        
    }

    ///
    /// Combination of systems with &mut A and &A. 
    /// Writers execute sequentially, readers in parallel
    /// 
    #[test]
    fn comp_sequential_parallel() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.run_system(|mut c: Commands| c.spawn(TestA(100)));

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestA| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_item: &mut TestA| {
            ptr.push(format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_item: &TestA| {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });

        let mut ptr = values.clone();
        app.system(Core, move |_item: &TestA| {
            ptr.push(format!("[C"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("C]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, A], [B, B], [C, [C, C], C]");
        
    }

    ///
    /// cycle of arrows with Mut<A> -> A and Mut<B> -> B forces planner to
    /// choose where to break the cycle
    /// 
    #[test]
    fn resmut_cycle() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());
        app.insert_resource(10 as u32);

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(Core, move |_r1: Res<u32>, _r2: ResMut<String>| {
            ptr.push(format!("[A"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("A]"));
        });
        
        let mut ptr = values.clone();
        app.system(Core, move |_r1: ResMut<u32>, _r2: Res<String>| {
            ptr.push(format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(format!("B]"));
        });

        app.tick();

        assert_eq!(values.take(), "[A, A], [B, B]");
    }

    struct ResA;
    struct ResB;
    struct ResC;


    #[allow(unused)]
    struct TestA(u32);
    #[allow(unused)]
    struct TestB(u32);
    // struct TestC(u32);

    impl Component for TestA {}
    impl Component for TestB {}
    // impl Component for TestC {}
}