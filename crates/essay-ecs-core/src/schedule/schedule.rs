use core::fmt;

use std::{hash::{Hash, Hasher}, collections::HashMap, any::Any, sync::mpsc};

use crate::{
    system::{SystemId, System}, 
    store::Store, 
    util::DynLabel, IntoSystemConfig};

use super::{
    phase::{IntoPhaseConfig, IntoPhaseConfigs, PhasePreorder, PhaseId, PhaseConfig, DefaultPhase, PhaseItem}, 
    Phase, 
    preorder::NodeId, 
    SystemMeta, 
    plan::Plan, 
    unsafe_cell::UnsafeSyncCell, 
    planner::Planner, 
    multithreaded::MultithreadedExecutor, UnsafeWorld, executor::{Executor, ExecutorFactory}, system::SystemConfig
};

///
/// See Bevy schedule.rs
/// 

pub type BoxedSystem<Out=()> = UnsafeSyncCell<Box<dyn System<Out=Out>>>;
pub type BoxedCondition<Out=bool> = UnsafeSyncCell<Box<dyn System<Out=Out>>>;
pub type BoxedLabel = Box<dyn ScheduleLabel>;

pub struct Schedules {
    schedule_map: HashMap<Box<dyn ScheduleLabel>, Schedule>,
    default_executor: Box<dyn ExecutorFactory>,
}

impl Schedules {
    pub fn get(
        &self, 
        label: impl AsRef<dyn ScheduleLabel>
    ) -> Option<&Schedule> {
        self.schedule_map.get(label.as_ref())
    }

    pub fn get_mut(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>
    ) -> Option<&mut Schedule> {
        self.schedule_map.get_mut(label.as_ref())
    }

    pub fn insert(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        schedule: Schedule
    ) -> Option<Schedule> {
        self.schedule_map.insert(label.as_ref().box_clone(), schedule)
    }

    pub fn remove(
        &mut self, 
        label: &dyn ScheduleLabel
    ) -> Option<Schedule> {
        self.schedule_map.remove(label)
    }

    pub fn remove_entry(
        &mut self, 
        label: &dyn ScheduleLabel
    ) -> Option<(BoxedLabel, Schedule)> {
        self.schedule_map.remove_entry(label)
    }

    pub fn add_system<M>(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        config: impl IntoSystemConfig<M>,
    ) {
        self.schedule_map.get_mut(label.as_ref())
            .unwrap_or_else(|| panic!("add_system with an unknown schedule {:?}", label.as_ref()))
            .add_system::<M>(config);
    }

    pub fn add_phases(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        config: impl IntoPhaseConfigs,
    ) {
        self.schedule_map.get_mut(label.as_ref())
            .unwrap_or_else(|| panic!("add_system with an unknown schedule {:?}", label.as_ref()))
            .add_phases(config);
    }

    pub fn set_executor(&mut self, executor: impl ExecutorFactory + 'static) {
        self.default_executor = Box::new(executor);

        for schedule in self.schedule_map.values_mut() {
            schedule.set_executor_factory(self.default_executor.box_clone());
        }
    }

    pub fn tick(
        &mut self, 
        label: impl AsRef<dyn ScheduleLabel>, 
        world: &mut Store
    ) -> Result<(), ScheduleErr> {
        let schedule = self.schedule_map.get_mut(label.as_ref()).unwrap();
        
        schedule.tick(world)
    }
}

impl Default for Schedules {
    fn default() -> Self {
        Self { 
            schedule_map: HashMap::new(),
            default_executor: Default::default(),
         }
    }
}

pub struct Schedule {
    inner: Option<ScheduleInner>,
    executor: Option<Box<dyn Executor>>,
}

impl Schedule {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_system<M>(
        &mut self, 
        config: impl IntoSystemConfig<M>
    ) {
        for system_cfg in config.into_config().systems {
            self.inner_mut().add_system(system_cfg);
        }
    }

    pub fn add_phase(&mut self, into_config: impl IntoPhaseConfig) { // -> PhaseItem {
        let config = into_config.into_config();

        let id = self.inner_mut().add_phase(config);
        /*
        self.init_phases();

        self.inner_mut().is_stale = true;

        self.inner().phases[id].clone()
        */
    }

    pub fn add_phases(&mut self, into_config: impl IntoPhaseConfigs) {
        let config = into_config.into_config();

        self.inner_mut().add_phases(config);
        //self.init_phases();

        //self.inner_mut().is_stale = true;
    }

    /*
    fn init_phases(&mut self) {
        let uninit = self.inner_mut().phases.uninit_phases();

        for phase_id in uninit {
            let first_id = self.add_system(
                PhaseSystem(phase_id)
            );

            let last_id = self.add_system(
                PhaseSystem(phase_id)
            );

            self.inner_mut().phases[phase_id].set_systems(first_id, last_id);
            // self.inner_mut().phases.set_system_id(phase_id, system_id);
        }
    }
    */

    pub fn tick(&mut self, world: &mut Store) -> Result<(), ScheduleErr> {
        while self.inner_mut().is_stale {
            self.inner_mut().is_stale = false;
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

        self.flush(world);

        Ok(())
    }

    pub(crate) fn init(&mut self, world: &mut Store) {
        self.inner_mut().init(world);

        /*
        self.init_phases();
        let phase_order = self.inner_mut().phases.sort();
        self.inner_mut().planner.sort(phase_order);
        */
    }

    pub(crate) fn plan(&mut self) -> Plan {
        self.inner_mut().plan()
        //let phase_order = self.inner().phases.sort();

        //self.inner().planner.plan(phase_order)
    }

    pub(crate) fn flush(&mut self, world: &mut Store) {
        self.inner_mut().flush(world);
    }

    pub(crate) unsafe fn run_system(&self, id: SystemId, world: &mut UnsafeWorld) {
        self.inner().systems[id.index()].as_mut().run(world);
    }

    pub(crate) unsafe fn run_unsafe(&self, id: SystemId, world: &UnsafeWorld) {
        self.inner().run_unsafe(id, world)
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

    pub fn set_executor(&mut self, executor: impl ExecutorFactory + 'static) {
        self.inner_mut().set_executor_factory(Box::new(executor));
    }

    fn set_executor_factory(&mut self, factory: Box<dyn ExecutorFactory>) {
        self.inner_mut().set_executor_factory(factory);
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Schedule {
            inner: Some(ScheduleInner {
                //phases: PhasePreorder::new(),

                systems: Default::default(),
                uninit_systems: Default::default(),
                conditions: Default::default(),

                planner: Planner::new(),

                executor_factory: Default::default(),
    
                is_stale: true,
            }),
            executor: None,
        }
    }
}

struct ScheduleInner {
    // phases: PhasePreorder,

    systems: Vec<BoxedSystem>,
    conditions: Vec<Vec<BoxedCondition>>,
    uninit_systems: Vec<SystemId>,

    planner: Planner,

    executor_factory: Box<dyn ExecutorFactory>,

    is_stale: bool,
}

impl ScheduleInner {
    fn add_system(&mut self, config: SystemConfig) {
        let SystemConfig {
            system,
            phases,
            mut conditions,
        } = config;

        let phase_ids = phases
            .iter()
            .map(|p| self.planner.add_phase(p))
            .collect();
        /*
        let phases = match phase {
            Some(phase) => {
                if phase == Box::new(DefaultPhase) {
                // self.inner_mut().phases.get_default_phase()
                    vec![]
                } else {
                    let phase_id = self.add_phase(
                    PhaseConfig::new(phase)
                    );
                    self.init_phases();
                    vec![phase_id]
                }
            }
            None => vec![],
        };
        */

        let phase_id = self.planner.phases_mut().add_phase_group(phase_ids);

        self.is_stale = true;

        self.add_system2(
            UnsafeSyncCell::new(system), 
            phase_id, 
            conditions.drain(..)
            .map(|s| UnsafeSyncCell::new(s))
            .collect(),
        );
    }

    fn add_system2(
        &mut self, 
        system: UnsafeSyncCell<Box<dyn System<Out = ()>>>, 
        phase_id: PhaseId,
        conditions: Vec<BoxedCondition>,
    ) -> SystemId {
        let id = SystemId(self.systems.len());
        let type_name = system.get_ref().type_name().to_string();

        self.systems.push(system);
        self.conditions.push(conditions);
        self.uninit_systems.push(id);

        self.planner.add(id, type_name, phase_id);

        self.is_stale = true;

        id
    }

    pub fn add_phase(&mut self, into_config: impl IntoPhaseConfig) { // -> PhaseItem {
        let config = into_config.into_config();

        let id = self.planner.phases_mut().add_phase(config);
        /*
        self.init_phases();

        self.inner_mut().is_stale = true;

        self.inner().phases[id].clone()
        */
    }

    fn add_phases(&mut self, into_config: impl IntoPhaseConfigs) {
        let config = into_config.into_config();

        self.planner.phases_mut().add_phases(config);
        self.init_phases();

        self.is_stale = true;
    }

    pub(crate) fn init(&mut self, world: &mut Store) {
        self.init_phases();

        for id in self.uninit_systems.drain(..) {
            let system = &mut self.systems[id.index()];
            let mut meta = self.planner.meta_mut(id);
            
            system.get_mut().init(&mut meta, world);

            for cond in &mut self.conditions[id.index()] {
                cond.get_mut().init(&mut meta, world);
            }
        }

        self.planner.sort();
    }

    fn init_phases(&mut self) {
        let uninit = self.planner.phases_mut().uninit_phases();

        for phase_id in uninit {
            let first_id = self.add_system2(
                UnsafeSyncCell::new(Box::new(PhaseSystem(phase_id))),
                PhaseId::zero(),
                Vec::new(),
            );

            let last_id = self.add_system2(
                UnsafeSyncCell::new(Box::new(PhaseSystem(phase_id))),
                PhaseId::zero(),
                Vec::new(),
            );

            self.planner.phases_mut()[phase_id].set_systems(first_id, last_id);
            // self.inner_mut().phases.set_system_id(phase_id, system_id);
        }
    }

    pub(crate) fn plan(&mut self) -> Plan {
        self.planner.plan()
    }

    pub(crate) fn flush(&mut self, world: &mut Store) {
        for system in &mut self.systems {
            system.get_mut().flush(world);
        }
    }

    fn set_executor_factory(&mut self, factory: Box<dyn ExecutorFactory>) {
        self.executor_factory = factory;
        self.is_stale = true;
    }
    /*
    fn set_phase(&mut self, phase_id: PhaseId, system_id: SystemId) {
        self.phases.set_system_id(phase_id, system_id);

        self.planner.meta_mut(system_id).set_phase(system_id);
    }
    */

    unsafe fn run_unsafe(&self, id: SystemId, world: &UnsafeWorld) {
        if self.conditions[id.index()].iter()
            .fold(true, |v, cond| {
            cond.as_mut().run_unsafe(world) && v
        }) {
            self.systems[id.index()].as_mut().run_unsafe(world);
        }
    }
}

#[derive(Debug)]
pub enum ScheduleErr {
    Misc,
    Err(Box<dyn Any + Send>),
    RecvErr(mpsc::RecvError),
    SendError,
    ParentPanic,
    ChildPanic,
}

struct PhaseSystem(PhaseId);

impl System for PhaseSystem {
    type Out = ();

    fn init(&mut self, meta: &mut SystemMeta, _world: &mut Store) {
        // meta.set_exclusive();
        meta.set_marker();
    }

    unsafe fn run_unsafe(&mut self, _world: &UnsafeWorld) -> Self::Out {
        panic!("PhaseSystem[{:?}] can't be called directly", self.0);
    }

    fn flush(&mut self, _world: &mut Store) {
    //    panic!("SystemFlush[{:?}] flush can't be called directly", self.0);
    }
}

pub trait ScheduleLabel : Send + DynLabel + fmt::Debug {
    fn box_clone(&self) -> BoxedLabel;
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

impl AsRef<dyn ScheduleLabel> for dyn ScheduleLabel {
    fn as_ref(&self) -> &dyn ScheduleLabel {
        self
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

#[cfg(test)]
mod tests {
    use crate::{store::Store, schedule::Phase, util::test::TestValues};

    use super::{Schedule, ScheduleLabel};
    use crate::*;

    mod ecs { pub mod core { pub use crate::*; }}
    use ecs as essay_ecs;


    #[test]
    fn schedule_label() {
        assert_eq!(format!("{:?}", TestSchedule::A), "A");
    }

    #[test]
    fn schedule_flush() {
        let mut world = Store::new();
        let mut schedule = Schedule::new();

        schedule.add_system(move |mut cmd: Commands| { 
            cmd.spawn(TestComp(2));
        });

        schedule.tick(&mut world).unwrap();
        assert_eq!(world.query::<&TestComp>()
            .map(|s| format!("comp{}", s.0))
            .collect::<Vec<String>>(), 
            vec!["comp2"]
        );
    }

    #[test]
    fn phase_a_b_c() {
        let mut values = TestValues::new();

        let mut world = Store::new();

        // A, default
        let mut schedule = new_schedule_a_b_c();

        let mut ptr = values.clone();
        schedule.add_system((move || { 
            ptr.push("a"); 
        }).phase(TestPhase::A));
        
        let mut ptr = values.clone();
        schedule.add_system(move || { 
            ptr.push("b"); 
        });

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "a, b");

        // C, default
        let mut schedule = new_schedule_a_b_c();

        let mut ptr = values.clone();
        schedule.add_system((move || { 
            ptr.push("c"); 
        }).phase(TestPhase::C));
        
        let mut ptr = values.clone();
        schedule.add_system(move || { 
            ptr.push("b"); 
        });

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "b, c");

        // default, A
        let mut schedule = new_schedule_a_b_c();

        let mut ptr = values.clone();
        schedule.add_system(move || { 
            ptr.push("b"); 
        });
        
        let mut ptr = values.clone();
        schedule.add_system((move || { 
            ptr.push("a"); 
        }).phase(TestPhase::A));

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "a, b");

        // default, C
        let mut schedule = new_schedule_a_b_c();

        let mut ptr = values.clone();
        schedule.add_system(move || { 
            ptr.push("b"); 
        });
        
        let mut ptr = values.clone();
        schedule.add_system((move || { 
            ptr.push("c"); 
        }).phase(TestPhase::C));

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "b, c");
    }

    fn new_schedule_a_b_c() -> Schedule {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhase::A,
            TestPhase::B,
            TestPhase::C,
        ).chain());
        // schedule.set_default_phase(TestPhase::B);

        schedule
    }

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

    #[derive(Component)]
    pub struct TestComp(u32);
}
