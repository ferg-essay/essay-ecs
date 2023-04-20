use core::fmt;

use std::{hash::{Hash, Hasher}, collections::HashMap};

use crate::{world::World, util::DynLabel};

use super::{
    phase::{IntoPhaseConfig, IntoPhaseConfigs, PhasePreorder, PhaseId, PhaseConfig, DefaultPhase}, 
    Phase, 
    preorder::{Preorder, NodeId}, 
    System, IntoSystemConfig, SystemConfig, SystemMeta, 
    plan::{PlanSystem, Plan}, 
    unsafe_cell::UnsafeSyncCell, 
    planner::{Planner, SystemItem}, 
    system::SystemId
};

///
/// See Bevy schedule.rs
/// 

pub type BoxedSystem<Out=()> = UnsafeSyncCell<Box<dyn System<Out=Out>>>;
pub type BoxedLabel = Box<dyn ScheduleLabel>;

pub struct Schedules {
    schedule_map: HashMap<Box<dyn ScheduleLabel>, Schedule>,
}

pub trait ScheduleLabel : DynLabel + fmt::Debug {
    fn box_clone(&self) -> BoxedLabel;
}

pub struct Schedule {
    inner: Option<ScheduleInner>,
    executor: Option<Box<dyn Executor>>,
}

pub trait Executor {
    fn run(
        &mut self, 
        schedule: Schedule, 
        world: World
    ) -> Result<(Schedule, World), ScheduleErr>;
}

pub trait ExecutorFactory {
    fn create(&self, plan: Plan) -> Box<dyn Executor>;
}

#[derive(Debug, Clone)]
pub struct ScheduleErr;

struct ScheduleInner {
    phases: PhasePreorder,

    systems: Vec<BoxedSystem>,
    uninit_systems: Vec<SystemId>,

    planner: Planner,

    executor_factory: Box<dyn ExecutorFactory>,

    is_changed: bool,
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
            schedule_map: HashMap::new(),
         }
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule {
            inner: Some(ScheduleInner {
                phases: PhasePreorder::new(),

                systems: Default::default(),
                uninit_systems: Default::default(),

                planner: Planner::new(),

                executor_factory: Default::default(),
    
                is_changed: true,
            }),
            executor: None,
        }
    }
}

impl Schedule {
    pub fn new() -> Self {
        Default::default()
    }
    /*
    pub(crate) fn system_mut(&mut self, system_id: SystemId) -> &mut SystemItem {
        self.inner_mut().planner.get_mut(system_id)
    }

    pub(crate) fn system(&self, system_id: SystemId) -> &SystemItem {
        self.inner().planner.get(system_id)
    }
    */

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
                    self.inner_mut().phases.get_default_phase()
                } else {
                    let phase_id = self.inner_mut().phases.add_phase(
                        PhaseConfig::new(phase)
                    );
                    self.init_phases();
                    Some(phase_id)
                }
            }
            None => None,
        };

        let phase_id = self.inner_mut().phases.get_server_id(phase_id);

        self.inner_mut().is_changed = true;

        self.inner_mut().add_system(UnsafeSyncCell::new(system), phase_id)
    }

    pub fn set_default_phase(&mut self, phase: impl Phase) {
        self.inner_mut().phases.set_default_phase(Box::new(phase));
    }

    pub fn add_phase(&mut self, into_config: impl IntoPhaseConfig) {
        let config = into_config.into_config();

        self.inner_mut().phases.add_phase(config);
        self.init_phases();

        self.inner_mut().is_changed = true;
    }

    pub fn add_phases(&mut self, into_config: impl IntoPhaseConfigs) {
        let config = into_config.into_config();

        self.inner_mut().phases.add_phases(config);
        self.init_phases();

        self.inner_mut().is_changed = true;
    }

    fn init_phases(&mut self) {
        let uninit = self.inner_mut().phases.uninit_phases();

        for phase_id in uninit {
            let system_id = self.add_system(
                SystemFlush(phase_id).no_phase()
            );

            self.inner_mut().phases.set_system_id(phase_id, system_id);
        }
    }

    pub fn run(&mut self, world: &mut World) -> Result<(), ScheduleErr> {
        while self.inner_mut().is_changed {
            self.inner_mut().is_changed = false;
            self.init(world);
            let plan = self.plan();
            self.executor = Some(
                self.inner_mut().executor_factory.create(plan)
            );
        }

        let exec_schedule = self.take();
        let exec_world = world.take();

        let executor = match &mut self.executor {
            Some(executor) => executor,
            None => { panic!("missing executor"); }
        };

        let (exec_schedule, exec_world) = executor.run(exec_schedule, exec_world)?;

        self.replace(exec_schedule);
        world.replace(exec_world);

        Ok(())
    }

    pub(crate) fn init(&mut self, world: &mut World) {
        self.inner_mut().init(world);

        self.init_phases();
        let phase_order = self.inner_mut().phases.sort();
        self.inner_mut().planner.sort(phase_order);
    }

    pub(crate) fn plan(&self) -> Plan {
        let phase_order = self.inner().phases.sort();

        self.inner().planner.plan(phase_order)
    }

    pub(crate) fn flush(&mut self, world: &mut World) {
        self.inner_mut().flush(world);
    }

    pub(crate) unsafe fn run_system(&self, id: SystemId, world: &mut World) {
        self.inner().systems[id.index()].as_mut().run(world);
    }

    pub(crate) unsafe fn run_unsafe(&self, id: SystemId, world: &World) {
        self.inner().systems[id.index()].as_mut().run_unsafe(world);
    }

    fn inner(&self) -> &ScheduleInner {
        match &self.inner {
            Some(inner) => inner,
            None => panic!("schedule has been taken for execution"),
        }
    }

    fn inner_mut(&mut self) -> &mut ScheduleInner {
        match &mut self.inner {
            Some(inner) => inner,
            None => panic!("schedule has been taken for execution"),
        }
    }

    pub(crate) fn take(&mut self) -> Self {
        Schedule{
            inner: self.inner.take(),
            executor: None,
        }
    }

    pub(crate) fn replace(&mut self, schedule: Schedule) {
        self.inner.replace(schedule.inner.unwrap());
    }

    pub(crate) fn meta(&self, id: SystemId) -> &SystemMeta {
        self.inner().planner.meta(id)
    }
}

impl ScheduleInner {
    pub(crate) fn init(&mut self, world: &mut World) {
        for id in self.uninit_systems.drain(..) {
            let system = &mut self.systems[id.index()];
            let mut meta = self.planner.meta_mut(id);
            
            system.get_mut().init(&mut meta, world);
        }
    }

    pub(crate) unsafe fn run_unsafe(&self, id: SystemId, world: &World) {
        let system = &self.systems[id.index()];

        system.as_mut().run_unsafe(world)
    }

    pub(crate) fn flush(&mut self, world: &mut World) {
        for system in &mut self.systems {
            //if ! system.meta.is_flush() {
                system.get_mut().flush(world);
            //}
        }
    }

    fn add_system(
        &mut self, 
        system: UnsafeSyncCell<Box<dyn System<Out = ()>>>, 
        phase_id: Option<SystemId>
    ) -> SystemId {
        let id = SystemId(self.systems.len());
        let type_name = system.get_ref().type_name().to_string();

        self.systems.push(system);
        self.uninit_systems.push(id);
        self.planner.add(id, type_name, phase_id);

        id
    }
}

struct SingleExecutorFactory;

impl ExecutorFactory for SingleExecutorFactory {
    fn create(&self, plan: Plan) -> Box<dyn Executor> {
        Box::new(SingleExecutor(plan))
    }
}

impl Default for Box<dyn ExecutorFactory> {
    fn default() -> Self {
        Box::new(SingleExecutorFactory {})
    }
}
struct SingleExecutor(Plan);

impl Executor for SingleExecutor {
    fn run(
        &mut self, 
        mut schedule: Schedule, 
        mut world: World
    ) -> Result<(Schedule, World), ScheduleErr> {
        for id in self.0.order() {
            let meta = schedule.meta(*id);

            if meta.is_flush() {
                schedule.flush(&mut world);
            }
            else {
                unsafe { schedule.run_system(*id, &mut world); }
            }
        }

        Ok((schedule, world))
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
    //    panic!("SystemFlush[{:?}] flush can't be called directly", self.0);
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
    use std::{rc::Rc, cell::RefCell, sync::{Arc, Mutex}};

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
        let values = Arc::new(Mutex::new(Vec::<String>::new()));

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

        schedule.run(&mut world).unwrap();
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

    fn take(values: &Arc<Mutex<Vec<String>>>) -> String {
        let str_vec = values.lock().unwrap().drain(..).collect::<Vec<String>>();

        return str_vec.join(", ");
    }

    fn push(values: &Arc<Mutex<Vec<String>>>, s: &str) {
        values.lock().unwrap().push(s.to_string());
    }
}
