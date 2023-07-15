use core::fmt;
use std::{
    any::type_name,
    collections::HashMap,
    hash::{Hash, Hasher}, ops::{Index, IndexMut},
};

use crate::{system::SystemId, util::DynLabel};

use super::preorder::{NodeId, Preorder};

///
/// See SystemSet in bevy_ecs/schedule/schedule.rs
///
/// renamed to phase because "set" is excessively abstract

pub trait Phase: Send + DynLabel + fmt::Debug {
    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }

    fn box_clone(&self) -> Box<dyn Phase>;
}

impl Phase for DefaultPhase {
    fn box_clone(&self) -> Box<dyn Phase> {
        Box::new(Clone::clone(self))
    }
}

pub struct PhaseConfig {
    phase: Box<dyn Phase>,
}

pub struct PhaseConfigs {
    phases: Vec<PhaseConfig>,
    is_chained: bool,
}

impl PhaseConfigs {
    fn new() -> PhaseConfigs {
        Self {
            phases: Vec::new(),
            is_chained: false,
        }
    }

    fn add(&mut self, config: PhaseConfig) {
        self.phases.push(config);
    }

    pub fn chained(mut self) -> PhaseConfigs {
        self.is_chained = true;
        self
    }
}

pub trait IntoPhaseConfig {
    fn into_config(self) -> PhaseConfig;
}

pub trait IntoPhaseConfigs: Sized {
    fn into_config(self) -> PhaseConfigs;

    fn chain(self) -> PhaseConfigs {
        self.into_config().chained()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct DefaultPhase;

//
// PhasePreorder
//

pub(crate) struct PhasePreorder {
    phase_map: HashMap<Box<dyn Phase>, PhaseId>,
    phases: Vec<PhaseItem>,
    preorder: Preorder,
}

impl PhasePreorder {
    pub fn new() -> Self {
        let mut preorder = Self {
            phase_map: HashMap::new(),
            phases: Vec::new(),
            preorder: Preorder::new(),
        };

        preorder.add_node(Box::new(DefaultPhase));

        preorder
    }

    pub fn add_phase(&mut self, config: PhaseConfig) -> PhaseId {
        let PhaseConfig { phase } = config;

        self.add_node(phase)
    }

    pub fn add_box_phase(&mut self, phase: &Box<dyn Phase>) -> PhaseId {
        self.add_node(phase.box_clone())
    }

    pub fn add_phases(&mut self, config: PhaseConfigs) {
        let PhaseConfigs {
            phases: sets,
            is_chained,
        } = config;

        let mut phase_iter = sets.into_iter();
        if is_chained {
            let Some(prev) = phase_iter.next() else { return };
            let mut prev_id = self.add_phase(prev);
            for next in phase_iter {
                let next_id = self.add_phase(next);

                self.preorder
                    .add_arrow(NodeId::from(prev_id), NodeId::from(next_id));

                prev_id = next_id;
            }
        } else {
            for phase in phase_iter {
                self.add_phase(phase);
            }
        }
    }

    fn add_node(&mut self, phase: Box<dyn Phase>) -> PhaseId {
        *self.phase_map.entry(phase.box_clone()).or_insert_with(|| {
            let node_id = self.preorder.add_node(0);
            let id = PhaseId::from(node_id);
            self.phases.push(PhaseItem {
                id,
                first_id: None,
                last_id: None,
            });
            id
        })
    }

    pub(crate) fn uninit_phases(&self) -> Vec<PhaseId> {
        self.phases
            .iter()
            .filter(|set| set.first_id.is_none())
            .map(|set| set.id)
            .collect()
    }

    pub(crate) fn sort(&self) -> Vec<PhaseId> {
        let mut preorder = self.preorder.clone();
        let order = preorder.sort();

        order
            .iter()
            .map(|id| PhaseId::from(*id))
            .collect()
    }

    pub(crate) fn add_phase_group(&self, phase_ids: Vec<PhaseId>) -> PhaseId {
        if phase_ids.len() == 0 {
            PhaseId::zero()
        } else if phase_ids.len() == 1 {
            phase_ids[0]
        } else {
            todo!()
        }
    }

    ///
    /// return SystemId of the phase markers with arrows into the phase
    /// 
    pub(crate) fn incoming_systems(&self, phase_id: PhaseId) -> Vec<SystemId> {
        self.preorder.incoming(NodeId::from(phase_id))
            .iter()
            .map(|n| { self.phases[n.0].last() })
            .collect::<Vec<SystemId>>()
    }
}

impl Index<PhaseId> for PhasePreorder {
    type Output = PhaseItem;

    fn index(&self, index: PhaseId) -> &Self::Output {
        &self.phases[index.0]
    }
}

impl IndexMut<PhaseId> for PhasePreorder {
    fn index_mut(&mut self, index: PhaseId) -> &mut Self::Output {
        &mut self.phases[index.0]
    }
}

//
// IntoPhaseConfig
//

impl PhaseConfig {
    pub fn new(phase: Box<dyn Phase>) -> Self {
        Self { phase }
    }
}
impl IntoPhaseConfig for PhaseConfig {
    fn into_config(self) -> PhaseConfig {
        self
    }
}

impl<T: Phase> IntoPhaseConfig for T {
    fn into_config(self) -> PhaseConfig {
        PhaseConfig::new(Box::new(self))
    }
}

impl IntoPhaseConfig for Box<dyn Phase> {
    fn into_config(self) -> PhaseConfig {
        PhaseConfig::new(self)
    }
}

impl IntoPhaseConfigs for PhaseConfigs {
    fn into_config(self) -> PhaseConfigs {
        self
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct PhaseId(usize);

impl PhaseId {
    pub fn index(&self) -> usize {
        self.0
    }

    pub(crate) fn zero() -> PhaseId {
        PhaseId(0)
    }
}

impl From<PhaseId> for NodeId {
    fn from(value: PhaseId) -> Self {
        NodeId(value.0)
    }
}

impl From<NodeId> for PhaseId {
    fn from(value: NodeId) -> Self {
        PhaseId(value.0)
    }
}

#[derive(Clone)]
pub struct PhaseItem {
    id: PhaseId,

    first_id: Option<SystemId>,
    last_id: Option<SystemId>,
}

impl PhaseItem {
    pub(crate) fn first(&self) -> SystemId {
        self.first_id.unwrap()
    }

    pub(crate) fn last(&self) -> SystemId {
        self.last_id.unwrap()
    }

    pub(crate) fn set_systems(&mut self, first_id: SystemId, last_id: SystemId) {
        assert!(self.first_id.is_none());
        assert!(self.last_id.is_none());

        self.first_id = Some(first_id);
        self.last_id = Some(last_id);
    }
}

impl PartialEq for dyn Phase {
    fn eq(&self, other: &Self) -> bool {
        self.dyn_eq(other.as_dyn_eq())
    }
}

impl Eq for dyn Phase {}

impl Hash for dyn Phase {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.dyn_hash(state);
    }
}

macro_rules! impl_task_set_tuple {
    ($($name:ident),*) => {
        #[allow(non_snake_case)]
        impl<$($name: IntoPhaseConfig,)*> IntoPhaseConfigs for ($($name,)*)
        {
            fn into_config(self) -> PhaseConfigs {
                let mut config = PhaseConfigs::new();
                let ($($name,)*) = self;
                $(
                    config.add($name.into_config());
                )*
                config
            }
        }
    }
}

//impl_task_set_tuple!();
impl_task_set_tuple!(P1);
impl_task_set_tuple!(P1, P2);
impl_task_set_tuple!(P1, P2, P3);
impl_task_set_tuple!(P1, P2, P3, P4);
impl_task_set_tuple!(P1, P2, P3, P4, P5);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6, P7);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6, P7, P8);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10);
impl_task_set_tuple!(P1, P2, P3, P4, P5, P6, P7, P8, P9, P10, P11);

#[cfg(test)]
mod tests {
    use essay_ecs_macros::Phase;

    use crate::{schedule::schedule::Schedule, util::test::TestValues, IntoPhaseConfigs, Store, IntoSystemConfig};
    use std::{
        thread,
        time::Duration,
    };

    use crate::{
        core_app::{Core, CoreApp},
        schedule::executor::Executors,
    };

    mod essay_ecs {
        pub mod core {
            pub mod schedule {
                pub use crate::schedule::*;
            }
        }
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
        }).phase(TestPhases::A));
        
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
        }).phase(TestPhases::C));
        
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
        }).phase(TestPhases::A));

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
        }).phase(TestPhases::C));

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "b, c");
    }

    #[test]
    fn phase_groups() {
        let mut app = CoreApp::new();

        app.set_executor(Executors::Multithreaded);
        app.insert_resource("test".to_string());

        let mut values = TestValues::new();

        let mut ptr = values.clone();
        app.system(
            Core,
            (move || {
                ptr.push(&format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr.push(&format!("C]"));
            })
            .phase(TestPhases::C),
        );

        let mut ptr = values.clone();
        app.system(
            Core,
            (move || {
                ptr.push(&format!("[C"));
                thread::sleep(Duration::from_millis(100));
                ptr.push(&format!("C]"));
            })
            .phase(TestPhases::C),
        );

        let mut ptr = values.clone();
        app.system(Core, move || {
            ptr.push(&format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(&format!("B]"));
        });

        let mut ptr = values.clone();
        app.system(Core, move || {
            ptr.push(&format!("[B"));
            thread::sleep(Duration::from_millis(100));
            ptr.push(&format!("B]"));
        });

        let mut ptr = values.clone();
        app.system(
            Core,
            (move || {
                ptr.push(&format!("[A"));
                thread::sleep(Duration::from_millis(100));
                ptr.push(&format!("A]"));
            })
            .phase(TestPhases::A),
        );

        let mut ptr = values.clone();
        app.system(
            Core,
            (move || {
                ptr.push(&format!("[A"));
                thread::sleep(Duration::from_millis(100));
                ptr.push(&format!("A]"));
            })
            .phase(TestPhases::A),
        );

        app.tick();

        assert_eq!(
            values.take(),
            "[A, [A, A], A], [B, [B, B], B], [C, [C, C], C]"
        );
    }

    #[test]
    fn default_vs_phase_groups() {
        let mut values = TestValues::new();

        let mut world = Store::new();

        // A, default
        let mut schedule = new_schedule_a_b_c();

        let mut ptr = values.clone();
        schedule.add_system((move || { 
            thread::sleep(Duration::from_millis(10));
            ptr.push(&format!("[A"));
            thread::sleep(Duration::from_millis(90));
            ptr.push(&format!("A]"));
        }).phase(TestPhases::A));
        
        let mut ptr = values.clone();
        schedule.add_system(move || { 
            ptr.push("[Def"); 
            thread::sleep(Duration::from_millis(90));
            ptr.push(&format!("Def]"));
        });

        schedule.tick(&mut world).unwrap();
        assert_eq!(values.take(), "a, b");
    }

    fn new_schedule_a_b_c() -> Schedule {
        let mut schedule = Schedule::new();
        schedule.add_phases((
            TestPhases::A,
            TestPhases::B,
            TestPhases::C,
        ).chain());
        //schedule.set_default_phase(TestPhases::B);

        schedule
    }

    #[derive(Phase, PartialEq, Hash, Eq, Clone, Debug)]
    enum TestPhases {
        A,
        B,
        C,
    }
}
