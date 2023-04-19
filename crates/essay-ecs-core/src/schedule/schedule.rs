use core::fmt;

use std::{hash::{Hash, Hasher}, collections::HashMap};

use crate::{world::World, util::DynLabel};

use super::{
    phase::{IntoPhaseConfig, IntoPhaseConfigs, PhasePreorder, PhaseId, PhaseConfig, DefaultPhase}, 
    Phase, 
    preorder::{Preorder, NodeId}, 
    System, IntoSystemConfig, SystemConfig, SystemMeta, plan::{PlanSystem, Plan}, unsafe_cell::UnsafeSyncCell, planner::Planner
};

///
/// See Bevy schedule.rs
/// 

pub type BoxedSystem<Out=()> = UnsafeSyncCell<Box<dyn System<Out=Out>>>;
pub type BoxedLabel = Box<dyn ScheduleLabel>;

pub struct Schedule {
    systems: Planner,

    phases: PhasePreorder,

    is_changed: bool,
}

pub struct Schedules {
    schedule: Schedule,

    schedule_map: HashMap<BoxedLabel, Schedule>,
}

pub trait ScheduleLabel : DynLabel + fmt::Debug {
    fn box_clone(&self) -> BoxedLabel;
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct SystemId(pub(crate) usize);

pub(crate) struct SystemItem {
    pub(crate) id: SystemId,
    pub(crate) meta: SystemMeta,

    pub(crate) system: BoxedSystem,
    pub(crate) phase: Option<SystemId>,
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

    pub(crate) unsafe fn run_unsafe(&self, world: &World) {
        self.system.as_mut().run_unsafe(world);
    }

    pub(crate) unsafe fn run(&self, world: &mut World) {
        self.system.as_mut().run(world);
    }

    pub(crate) fn system(&self) -> &BoxedSystem {
        &self.system
    }
}

impl Schedule {
    pub fn new() -> Self {
        Schedule {
            systems: Default::default(),

            phases: PhasePreorder::new(),

            is_changed: true,
        }
    }

    pub(crate) fn system_mut(&mut self, system_id: SystemId) -> &mut SystemItem {
        self.systems.get_mut(system_id)
    }

    pub(crate) fn system(&self, system_id: SystemId) -> &SystemItem {
        self.systems.get(system_id)
    }

    pub fn add_system<M>(
        &mut self, 
        config: impl IntoSystemConfig<M>
    ) -> SystemId {
        let SystemConfig {
            system,
            phase,
        } = config.into_config();

        let phase_id = match phase {
            Some(phase) => {
                if phase == Box::new(DefaultPhase) {
                    self.phases.get_default_phase()
                } else {
                    let phase_id = self.phases.add_phase(
                        PhaseConfig::new(phase)
                    );
                    self.init_phases();
                    Some(phase_id)
                }
            }
            None => None,
        };

        let phase_id = self.phases.get_server_id(phase_id);

        self.is_changed = true;

        self.systems.add(UnsafeSyncCell::new(system), phase_id)
    }

    pub fn set_default_phase(&mut self, phase: impl Phase) {
        self.phases.set_default_phase(Box::new(phase));
    }

    pub fn add_phase(&mut self, into_config: impl IntoPhaseConfig) {
        let config = into_config.into_config();

        self.phases.add_phase(config);
        self.init_phases();

        self.is_changed = true;
    }

    pub fn add_phases(&mut self, into_config: impl IntoPhaseConfigs) {
        let config = into_config.into_config();

        self.phases.add_phases(config);
        self.init_phases();

        self.is_changed = true;
    }

    fn init_phases(&mut self) {
        let uninit = self.phases.uninit_phases();

        for phase_id in uninit {
            let system_id = self.add_system(
                SystemFlush(phase_id).no_phase()
            );

            self.phases.set_system_id(phase_id, system_id);
        }
    }

    pub fn run(&mut self, world: &mut World) {
        while self.is_changed {
            self.is_changed = false;
            self.init(world);
        }

        self.systems.run(world);
        self.systems.flush(world);
    }

    pub(crate) fn init(&mut self, world: &mut World) {
        self.systems.init(world);
        self.init_phases();
        let phase_order = self.phases.sort();
        self.systems.sort(phase_order);
    }

    pub(crate) fn plan(&self) -> Plan {
        let phase_order = self.phases.sort();

        self.systems.plan(phase_order)
    }

    pub(crate) fn flush(&mut self, world: &mut World) {
        self.systems.flush(world);
    }

    pub(crate) unsafe fn run_unsafe(&self, id: SystemId, world: &World) {
        self.systems.run_unsafe(id, world);
    }
}

impl Schedules {
    pub fn get(&self, label: &dyn ScheduleLabel) -> Option<&Schedule> {
        self.schedule_map.get(label)
    }

    pub fn insert(&mut self, label: impl ScheduleLabel, schedule: Schedule) -> Option<Schedule> {
        self.schedule_map.insert(label.box_clone(), schedule)
    }

    pub fn add_system<M>(
        &mut self, 
        label: &dyn ScheduleLabel, 
        config: impl IntoSystemConfig<M>,
    ) {
        self.schedule_map.get_mut(label)
            .unwrap_or_else(|| panic!("add_system with an unknown schedule {:?}", label))
            .add_system::<M>(config);
    }

    pub fn run(&mut self, label: &dyn ScheduleLabel, world: &mut World) {
        let (key, mut schedule) = self.schedule_map.remove_entry(label).unwrap();
        
        schedule.run(world);

        self.schedule_map.insert(key, schedule);
    }

    pub(crate) fn remove(
        &mut self, 
        label: &dyn ScheduleLabel
    ) -> Option<Schedule> {
        self.schedule_map.remove(label)
    }

    pub(crate) fn remove_entry(
        &mut self, 
        label: &dyn ScheduleLabel
    ) -> Option<(BoxedLabel, Schedule)> {
        self.schedule_map.remove_entry(label)
    }
}

impl Default for Schedules {
    fn default() -> Self {
        Self { 
            schedule: Schedule::new(),
            schedule_map: HashMap::new(),
         }
    }
}

struct SystemFlush(PhaseId);

impl System for SystemFlush {
    type Out = ();

    fn init(&mut self, meta: &mut SystemMeta, _world: &mut World) {
        meta.set_exclusive();
        meta.set_flush();
    }

    unsafe fn run_unsafe(&mut self, _world: &World) -> Self::Out {
        panic!("SystemFlush[{:?}] run_unsafe can't be called directly", self.0);
    }

    fn flush(&mut self, _world: &mut World) {
        panic!("SystemFlush[{:?}] flush can't be called directly", self.0);
    }
}

impl SystemItem {
    pub(crate) fn meta(&self) -> &SystemMeta {
        &self.meta
    }
}

impl SystemId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl From<NodeId> for SystemId {
    fn from(value: NodeId) -> Self {
        SystemId(value.index())
    }
}

impl From<SystemId> for NodeId {
    fn from(value: SystemId) -> Self {
        NodeId(value.index())
    }
}

impl PartialEq for dyn ScheduleLabel {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_dyn_eq())
    }
}

impl Eq for dyn ScheduleLabel {}

impl Hash for dyn ScheduleLabel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

#[cfg(test)]
mod tests {
    use std::{rc::Rc, cell::RefCell};

    use crate::{world::World, schedule::Phase};

    use super::{Schedule, ScheduleLabel};
    use crate::*;

    #[derive(PartialEq, Hash, Eq, Clone, Debug)]
    enum TestSchedule {
        A,
    }

    impl ScheduleLabel for TestSchedule {
        fn box_clone(&self) -> Box<dyn ScheduleLabel> {
            Box::new(Clone::clone(self))
        }
    }

    #[derive(PartialEq, Hash, Eq, Clone, Debug)]
    enum TestPhase {
        A,
        B,
        C,
    }

    impl Phase for TestPhase {
        fn box_clone(&self) -> Box<dyn Phase> {
            Box::new(Clone::clone(self))
        }
    }

    #[test]
    fn schedule_label() {
        assert_eq!(format!("{:?}", TestSchedule::A), "A");
    }

    #[test]
    fn phase_a_b_c() {
        /*
        let values = Rc::new(RefCell::new(Vec::<String>::new()));

        let mut world = World::new();

        // A, default
        let mut schedule = new_schedule_a_b_c();

        let ptr = values.clone();
        schedule.add_system((move || { 
            push(&ptr, "a"); 
        }).phase(TestPhase::A));
        
        let ptr = values.clone();
        schedule.add_system(move || { 
            push(&ptr, "b"); 
        });

        schedule.run(&mut world);
        assert_eq!(take(&values), "a, b");

        // C, default
        let mut schedule = new_schedule_a_b_c();

        let ptr = values.clone();
        schedule.add_system((move || { 
            push(&ptr, "c"); 
        }).phase(TestPhase::C));
        
        let ptr = values.clone();
        schedule.add_system(move || { 
            push(&ptr, "b"); 
        });

        schedule.run(&mut world);
        assert_eq!(take(&values), "b, c");

        // default, A
        let mut schedule = new_schedule_a_b_c();

        let ptr = values.clone();
        schedule.add_system(move || { 
            push(&ptr, "b"); 
        });
        
        let ptr = values.clone();
        schedule.add_system((move || { 
            push(&ptr, "a"); 
        }).phase(TestPhase::A));

        schedule.run(&mut world);
        assert_eq!(take(&values), "a, b");

        // default, C
        let mut schedule = new_schedule_a_b_c();

        let ptr = values.clone();
        schedule.add_system(move || { 
            push(&ptr, "b"); 
        });
        
        let ptr = values.clone();
        schedule.add_system((move || { 
            push(&ptr, "c"); 
        }).phase(TestPhase::C));

        schedule.run(&mut world);
        assert_eq!(take(&values), "b, c");
        */
    }

    fn new_schedule_a_b_c() -> Schedule {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhase::A,
            TestPhase::B,
            TestPhase::C,
        ).chained());
        schedule.set_default_phase(TestPhase::B);

        schedule
    }

    fn test_a() {
        println!("a");
    }

    fn test_b() {
        println!("b");
    }

    fn take(values: &Rc<RefCell<Vec<String>>>) -> String {
        let str_vec = values.borrow_mut().drain(..).collect::<Vec<String>>();

        return str_vec.join(", ");
    }

    fn push(values: &Rc<RefCell<Vec<String>>>, s: &str) {
        values.borrow_mut().push(s.to_string());
    }
}
