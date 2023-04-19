use std::collections::HashMap;

use crate::{schedule::SystemMeta, World};

use super::{schedule::{SystemItem, BoxedSystem}, preorder::{Preorder, NodeId}, plan::Plan, system::SystemId};


pub struct Planner {
    systems: Vec<SystemItem>,

    uninit_systems: Vec<SystemId>,

    preorder: Preorder,

    order: Vec<SystemId>,
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
        system: BoxedSystem,
        phase_id: Option<SystemId>,
    ) -> SystemId {
        // let system: BoxedSystem = Box::new(IntoSystem::into_system(system));

        let id = self.preorder.add_node(0);
        assert_eq!(id.index(), self.systems.len());

        let id = SystemId::from(id);

        self.systems.push(SystemItem {
            id,
            meta: SystemMeta::new(id, system.get_ref().type_name()),
            system,
            phase: phase_id,
        });

        self.uninit_systems.push(id);

        id
    }

    pub(crate) fn init(&mut self, world: &mut World) {
        for id in self.uninit_systems.drain(..) {
            let system = &mut self.systems[id.index()];
            //println!("init {:?}", id);
            system.system.get_mut().init(&mut system.meta, world);
        }
    }

    pub(crate) fn sort(&mut self, phase_order: Vec<SystemId>) {
        let mut preorder = self.preorder.clone();

        let prev_map = self.prev_map(
            &mut preorder, 
            phase_order
        );

        for system in &self.systems {
            if ! system.meta.is_flush() {
                system.add_phase_arrows(&mut preorder, &prev_map);
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

        for system in &self.systems {
            if ! system.meta.is_flush() {
                system.add_phase_arrows(&mut preorder, &prev_map);
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

    pub(crate) unsafe fn run_unsafe(&self, id: SystemId, world: &World) {
        let system = &self.systems[id.index()].system;

        system.as_mut().run_unsafe(world)
    }

    pub(crate) fn flush(&mut self, world: &mut World) {
        for system in &mut self.systems {
            if ! system.meta.is_flush() {
                system.system.get_mut().flush(world);
            }
        }
    }

    pub(crate) fn get_mut(&mut self, system_id: SystemId) -> &mut SystemItem {
        &mut self.systems[system_id.index()]
    }

    pub(crate) fn get(&self, system_id: SystemId) -> &SystemItem {
        &self.systems[system_id.index()]
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
