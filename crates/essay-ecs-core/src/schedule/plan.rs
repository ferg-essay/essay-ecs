use super::{schedule::SystemId, preorder::{Preorder, NodeId}};

pub struct Plan {
    systems: Vec<SystemPlan>,
    n_incoming: Vec<usize>,
    n_initial: usize,
}

pub struct SystemPlan {
    id: SystemId,
    n_incoming: usize,
    outgoing: Vec<SystemId>,
}

impl Plan {
    pub fn new(preorder: &Preorder) -> Self {
        let systems: Vec<SystemPlan> = preorder
            .sort()
            .iter()
            .map(|n| SystemPlan::new(preorder, *n))
            .collect();

        Self {
            n_incoming: systems.iter()
                .map(|s| s.n_incoming)
                .collect(),
            n_initial: systems.iter()
                .filter(|p| p.n_incoming == 0)
                .count(),
            systems: systems,
        }
    }

    pub fn len(&self) -> usize {
        self.systems.len()
    }

    pub fn n_initial(&self) -> usize {
        self.n_initial
    }

    pub fn n_incoming(&self) -> &Vec<usize> {
        &self.n_incoming
    }

    pub fn system_id(&self, i: usize) -> SystemId {
        self.systems[i].id
    }
}

impl SystemPlan {
    fn new(preorder: &Preorder, id: NodeId) -> Self {
        Self {
            id: SystemId::from(id),
            n_incoming: preorder.incoming(id).len(),
            outgoing: preorder.outgoing(id).iter()
                .map(|n| SystemId::from(*n))
                .collect(),
        }
    }
}
