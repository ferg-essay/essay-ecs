use crate::system::SystemId;

use super::{preorder::{Preorder, NodeId}};

pub struct Plan {
    systems: Vec<PlanSystem>,

    order: Vec<SystemId>,
    n_incoming: Vec<usize>,
}

#[derive(Debug)]
pub struct PlanSystem {
    n_incoming: usize,
    outgoing: Vec<usize>,
}

impl Plan {
    pub fn new(preorder: &Preorder) -> Self {
        let order = preorder.sort();
        let system_order: Vec<SystemId> = order.iter()
            .map(|n| SystemId::from(*n))
            .collect();

        let systems : Vec<PlanSystem> = preorder.node_ids()
            .iter()
            .map(|n| PlanSystem::new(
                preorder, 
                *n,
                &order
            )).collect();

        let n_incoming: Vec<usize> = system_order.iter()
                .map(|s| systems[s.index()].n_incoming)
                .collect();
            
        Self {
            order: system_order,
            systems,
            n_incoming,
        }
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn order(&self) -> &Vec<SystemId> {
        &self.order
    }

    pub fn n_incoming(&self) -> &Vec<usize> {
        &self.n_incoming
    }

    pub fn system_id(&self, i: usize) -> SystemId {
        self.order[i]
    }

    pub(crate) fn outgoing(&self, id: SystemId) -> &Vec<usize> {
        &self.systems[id.index()].outgoing
    }
}

impl PlanSystem {
    fn new(
        preorder: &Preorder, 
        id: NodeId,
        order: &Vec<NodeId>) -> Self {
        Self {
            n_incoming: preorder.incoming(NodeId::from(id)).len(),
            outgoing: preorder.outgoing(NodeId::from(id)).iter()
                .map(|n|
                    order.iter().position(|n2| n == n2).unwrap()
                ).collect(),
        }
    }
}

