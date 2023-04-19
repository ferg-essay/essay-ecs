use core::fmt;
use std::{hash::{Hash, Hasher}, any::type_name, collections::HashMap};

use crate::{util::DynLabel};

use super::{preorder::{Preorder, NodeId}, system::{SystemId}};

///
/// See SystemSet in bevy_ecs/schedule/schedule.rs
/// 

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct PhaseId(usize);

pub trait Phase : DynLabel + fmt::Debug {
    fn name(&self) -> String {
        type_name::<Self>().to_string()
    }

    fn box_clone(&self) -> Box<dyn Phase>;
}

pub struct PhaseItem {
    id: PhaseId,
    phase: Box<dyn Phase>,

    system_id: Option<SystemId>,
    prev: Option<PhaseId>,
}

pub(crate) struct PhasePreorder {
    phase_map: HashMap<Box<dyn Phase>, PhaseId>,
    phases: Vec<PhaseItem>,
    default_phase: Option<PhaseId>,
    preorder: Preorder,
}

pub struct PhaseConfig {
    phase: Box<dyn Phase>,
}

pub struct PhaseConfigs {
    sets: Vec<PhaseConfig>,
    is_chained: bool,
}

pub trait IntoPhaseConfig {
    fn into_config(self) -> PhaseConfig;
}

pub trait IntoPhaseConfigs: Sized {
    fn into_config(self) -> PhaseConfigs;

    fn chained(self) -> PhaseConfigs {
        self.into_config().chained()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub struct DefaultPhase;

impl Phase for DefaultPhase {
    fn box_clone(&self) -> Box<dyn Phase> {
        Box::new(Clone::clone(self))
    }
}


//
// TaskSetMeta
//

impl PhasePreorder {
    pub fn new() -> Self {
        Self {
            phase_map: HashMap::new(),
            phases: Vec::new(),
            default_phase: None,
            preorder: Preorder::new(),
        }
    }

    pub fn add_phase(&mut self, config: PhaseConfig) -> PhaseId {
        let PhaseConfig {
            phase,
        } = config; 

        self.add_node(phase)
    }

    pub fn add_phases(&mut self, config: PhaseConfigs) {
        let PhaseConfigs { sets, is_chained } = config;

        let mut set_iter = sets.into_iter();
        if is_chained {
            let Some(prev) = set_iter.next() else { return };
            let mut prev_id = self.add_phase(prev);
            for next in set_iter {
                let next_id = self.add_phase(next);

                self.preorder.add_arrow(
                    NodeId::from(prev_id), 
                    NodeId::from(next_id)
                );

                prev_id = next_id;
            }
        } else {
            for set in set_iter {
                self.add_phase(set);
            }
        }
    }

    pub fn set_default_phase(&mut self, task_set: Box<dyn Phase>) {
        let id = self.add_node(task_set);

        self.default_phase = Some(id);
    }

    pub fn get_default_phase(&self) -> Option<PhaseId> {
        self.default_phase
    }

    fn add_node(&mut self, phase: Box<dyn Phase>) -> PhaseId {
        *self.phase_map
            .entry(phase.box_clone())
            .or_insert_with(|| {
            let node_id = self.preorder.add_node(0);
            let id = PhaseId::from(node_id);
            self.phases.push(PhaseItem {
                id,
                phase,
                prev: None,
                system_id: None,
            });
            id
        })
    }

    pub(crate) fn uninit_phases(&self) -> Vec<PhaseId> {
        self.phases.iter()
            .filter(|set| set.system_id.is_none())
            .map(|set| set.id)
            .collect()
    }

    pub(crate) fn set_system_id(
        &mut self, 
        phase_id: PhaseId, 
        system_id: SystemId
    ) {
        assert!(self.phases[phase_id.index()].system_id.is_none());

        self.phases[phase_id.index()].system_id = Some(system_id);
    }

    pub(crate) fn sort(&self) -> Vec<SystemId> {
        let order = self.preorder.sort();

        order.iter()
            .map(|id| self.phases[id.index()].system_id.unwrap())
            .collect()
    }

    pub(crate) fn get_server_id(&self, phase_id: Option<PhaseId>) -> Option<SystemId> {
        match phase_id {
            Some(phase_id) => self.phases[phase_id.0].system_id,
            None => None,
        }
    }
}

impl PhaseConfigs {
    fn new() -> PhaseConfigs {
        Self {
            sets: Vec::new(),
            is_chained: false,
        }
    }

    fn add(&mut self, config: PhaseConfig) {
        self.sets.push(config);
    }

    pub fn chained(mut self) -> PhaseConfigs {
        self.is_chained = true;
        self
    }
}
//
// IntoTaskSetConfig
//

impl PhaseConfig {
    pub fn new(phase: Box<dyn Phase>) -> Self {
        Self {
            phase
        }
    }
}
impl IntoPhaseConfig for PhaseConfig {
    fn into_config(self) -> PhaseConfig {
        self
    }
}

impl<T:Phase> IntoPhaseConfig for T {
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

impl PhaseId {
    pub fn index(&self) -> usize {
        self.0
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

impl_task_set_tuple!();
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
    use crate::schedule::schedule::Schedule;

    use super::Phase;

    #[test]
    fn add_default_task_set() {
        let mut schedule = Schedule::new();
        schedule.set_default_phase(TestSet::A);
    }

    #[derive(Clone, PartialEq, Eq, Hash, Debug)]
    enum TestSet {
        A,
    }

    impl Phase for TestSet {
        fn box_clone(&self) -> Box<dyn Phase> {
            Box::new(Clone::clone(self))
        }
    }
}