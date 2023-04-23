use core::fmt;
use std::{collections::HashSet, cmp::{Ordering}};

use fixedbitset::FixedBitSet;
use log::info;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub(crate) usize);

#[derive(Clone)]
pub struct Preorder {
    nodes: Vec<Node>,
}

#[derive(Clone)]
struct Node {
    id: NodeId,

    weight: u64, // greedy value

    incoming: HashSet<NodeId>,
    outgoing: HashSet<NodeId>,
}

impl Preorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn incoming(&self, id: NodeId) -> &HashSet<NodeId> {
        &self.nodes[id.index()].incoming
    }

    pub(crate) fn outgoing(&self, id: NodeId) -> &HashSet<NodeId> {
        &self.nodes[id.index()].outgoing
    }

    pub fn add_node(&mut self, weight: u64) -> NodeId {
        let id = NodeId(self.nodes.len());

        self.nodes.push(Node::new(id, weight));

        id
    }

    pub(crate) fn node_ids(&self) -> Vec<NodeId> {
        self.nodes.iter().map(|n| n.id()).collect()
    }

    pub fn add_arrow(&mut self, source_id: NodeId, target_id: NodeId) {
        assert_ne!(source_id, target_id);

        self.nodes[source_id.0].outgoing.insert(target_id);
        self.nodes[target_id.0].incoming.insert(source_id);

        //println!("Arrow[{:?}] out:{:?}", source_id, self.nodes[source_id.0].outgoing);
    }

    pub fn sort(&mut self) -> Vec<NodeId> {
        let mut results = Vec::<NodeId>::new();

        let mut pending = FixedBitSet::with_capacity(self.nodes.len());
        pending.insert_range(..);

        let mut completed = FixedBitSet::with_capacity(self.nodes.len());

        while results.len() < self.nodes.len() {
            let start_len = results.len();
            //println!("  Loop: {}", start_len);

            completed.clear();

            for index in pending.ones() {
                let node = &self.nodes[index];

                if ! node.is_incoming_pending(&pending) {
                    completed.insert(index);
                    results.push(node.id());
                    //println!("   Item: {:?}", node.id());
                }
            }

            if results.len() == start_len {
                self.break_cycle(&pending);
            }

            let new_results = &mut results.as_mut_slice()[start_len..];
            
            new_results.sort_by_key(|n| u64::MAX - self.nodes[n.0].weight);

            pending.difference_with(&completed);
        }

        assert!(results.len() == self.nodes.len());
        results
    }

    fn break_cycle(&mut self, pending: &FixedBitSet) {
        let mut cycle_ids : Vec<NodeId> = pending.ones()
            .map(|i| NodeId(i))
            .filter(|n| self.is_cyclic(*n, &pending))
            .collect();
        
        cycle_ids.sort_by(|&a, &b| {
            self.compare_nodes(a, b, pending)
        });

        let node_id = cycle_ids[0];

        info!("breaking cycle with {:?}", self.nodes[node_id.index()]);

        while self.remove_pending(node_id, pending) {
        }
        //panic!("preorder sort unable to make progress, possibly due to loops");
    }

    fn remove_pending(&mut self, node_id: NodeId, pending: &FixedBitSet) -> bool {
        let node = &mut self.nodes[node_id.index()];

        let incoming_id = match node.find_pending(&pending) {
            Some(incoming_id) => incoming_id,
            None => return false,
        };

        node.remove_incoming(incoming_id);
        self.nodes[incoming_id.index()].remove_outgoing(node_id);
        
        return false;
    }

    fn compare_nodes(
        &self, 
        id_a: NodeId, 
        id_b: NodeId, 
        pending: &FixedBitSet
    ) -> Ordering {
        let node_a = &self.nodes[id_a.index()];
        let node_b = &self.nodes[id_b.index()];

        let is_path_a_to_b = self.is_path_to(id_a, id_b, pending);
        let is_path_b_to_a = self.is_path_to(id_b, id_a, pending);

        if is_path_a_to_b && ! is_path_b_to_a {
            return Ordering::Less;
        } else if is_path_b_to_a && ! is_path_a_to_b {
            return Ordering::Greater;
        }

        let cmp = node_a.incoming.len().cmp(&node_b.incoming.len());
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = node_b.outgoing.len().cmp(&node_a.outgoing.len());
        if cmp != Ordering::Equal {
            return cmp;
        }

        let cmp = node_a.weight.cmp(&node_b.weight);
        if cmp != Ordering::Equal {
            return cmp;
        }

        id_a.cmp(&id_b)
    }

    fn is_cyclic(&self, id: NodeId, pending: &FixedBitSet) -> bool {
        let mut visited = HashSet::<NodeId>::new();
        visited.insert(id);

        self.is_cyclic_rec(id, id, pending, &mut visited)
    }

    fn is_cyclic_rec(
        &self, 
        top_id: NodeId, 
        id: NodeId, 
        pending: &FixedBitSet,
        visited: &mut HashSet<NodeId>,
    ) -> bool {
        let node = &self.nodes[id.index()];
        
        for out_id in &node.outgoing {
            if *out_id == top_id {
                return true;
            } else if ! visited.contains(out_id) {
                visited.insert(*out_id);

                if self.is_cyclic_rec(top_id, *out_id, pending, visited) {
                    return true;
                }
            }
        }

        false
    }

    fn is_path_to(
        &self, 
        id_a: NodeId, 
        id_b: NodeId,
        pending: &FixedBitSet
    ) -> bool {
        self.is_path_to_rec(id_a, id_b, id_a, pending, &mut HashSet::new())
    }

    fn is_path_to_rec(
        &self, 
        id_a: NodeId, 
        id_b: NodeId,
        id: NodeId,
        pending: &FixedBitSet,
        visited: &mut HashSet<NodeId>,
    ) -> bool {
        if id == id_b {
            return true;
        } else if visited.contains(&id) {
            return false;
        } else {
            visited.insert(id);

            let node = &self.nodes[id.index()];
            for id_out in &node.outgoing {
                if self.is_path_to_rec(id_a, id_b, *id_out, pending, visited) {
                    return true;
                }
            }

            return false;
        }

    }
}

impl Default for Preorder {
    fn default() -> Self {
        Self { nodes: Default::default() }
    }
}

impl NodeId {
    pub fn index(&self) -> usize {
        self.0
    }
}

impl Node {
    fn new(id: NodeId, weight: u64) -> Self {
        Self {
            id,
            weight,
            incoming: Default::default(),
            outgoing: Default::default(),
        }
    }

    fn id(&self) -> NodeId {
        self.id
    }

    fn is_incoming_pending(&self, pending: &FixedBitSet) -> bool {
        for incoming in &self.incoming {
            if pending.contains(incoming.0) {
                return true;
            }
        }

        return false
    }

    fn remove_incoming(&mut self, node_id: NodeId) {
        self.incoming.remove(&node_id);
    }

    fn remove_outgoing(&mut self, node_id: NodeId) {
        self.outgoing.remove(&node_id);
    }

    fn find_pending(&mut self, pending: &FixedBitSet) -> Option<NodeId> {
        for incoming in &self.incoming {
            if pending.contains(incoming.0) {
                return Some(*incoming);
            }
        }

        return None
    }
}

impl fmt::Debug for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Node")
            .field("id", &self.id)
            .field("weight", &self.weight)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{Preorder, NodeId};

    #[test]
    fn empty() {
        let mut graph = Preorder::new();

        assert_eq!(as_vec(graph.sort()).as_slice(), []);
    }

    #[test]
    fn no_arrows() {
        let mut graph = Preorder::new();

        let n0 = graph.add_node(0);
        assert_eq!(n0, NodeId(0));

        let n1 = graph.add_node(0);
        assert_eq!(n1, NodeId(1));

        let n2 = graph.add_node(0);
        assert_eq!(n2, NodeId(2));

        let n3 = graph.add_node(0);
        assert_eq!(n3, NodeId(3));

        assert_eq!(as_vec(graph.sort()).as_slice(), [0, 1, 2, 3]);
    }

    #[test]
    fn pair() {
        let mut g = graph(2, &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1]);

        let mut g = graph(2, &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0]);
    }

    #[test]
    fn triple() {
        // single arrows
        let mut g = graph(3, &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        let mut g = graph(3, &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let mut g = graph(3, &[(0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let mut g = graph(3, &[(2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let mut g = graph(3, &[(1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let mut g = graph(3, &[(2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        // two arrows
        let mut g = graph(3, &[(0, 1), (0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let mut g = graph(3, &[(0, 1), (2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);

        let mut g = graph(3, &[(1, 0), (2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let mut g = graph(3, &[(1, 0), (0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0, 2]);

        // --
        let mut g = graph(3, &[(0, 1), (1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let mut g = graph(3, &[(1, 0), (1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0, 2]);

        let mut g = graph(3, &[(1, 0), (2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let mut g = graph(3, &[(0, 1), (2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);
    }

    #[test]
    #[should_panic]
    fn cycle() {
        let mut g = graph(3, &[(0, 1), (1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);
    }

    #[test]
    fn weights_no_arrows() {
        let mut g = graph_w(&[0, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0]);

        let mut g = graph_w(&[1, 0], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1]);

        let mut g = graph_w(&[0, 1, 2], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let mut g = graph_w(&[0, 2, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let mut g = graph_w(&[0, 0, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);
    }

    #[test]
    fn weights_triple_one_arrow() {
        let mut g = graph_w(&[0, 1, 2], &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);

        let mut g = graph_w(&[0, 1, 2], &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let mut g = graph_w(&[2, 1, 0], &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        let mut g = graph_w(&[2, 1, 0], &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);
    }

    #[test]
    fn weights_dual_cycle() {
        let mut g = graph(4, &[
            (0, 1),
            (1, 0),
            (2, 3),
            (3, 2),
            (2, 0),
        ]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 3, 1, 0]);

        let mut g = graph(4, &[
            (0, 1),
            (1, 0),
            (2, 3),
            (3, 2),
            (0, 2),
        ]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 3, 2]);

        let mut g = graph(6, &[
            (1, 0),
            (1, 2),
            (2, 1),

            (3, 2),

            (3, 4),
            (4, 3),
            (5, 3)
        ]);
        assert_eq!(as_vec(g.sort()).as_slice(), [5, 4, 3, 1, 0, 2]);

        let mut g = graph(6, &[
            (0, 1),
            (1, 2),
            (2, 1),

            (2, 3),

            (3, 4),
            (4, 3),
            (4, 5)
        ]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1, 4, 3, 5]);

    }

    fn graph(n: usize, arrows: &[(usize, usize)]) -> Preorder {
        let mut graph = Preorder::new();

        for _ in 0..n {
            graph.add_node(0);
        }

        for arrow in arrows {
            graph.add_arrow(NodeId(arrow.0), NodeId(arrow.1));
        }

        graph
    }

    fn graph_w(weights: &[u64], arrows: &[(usize, usize)]) -> Preorder {
        let mut graph = Preorder::new();

        for weight in weights {
            graph.add_node(*weight);
        }

        for arrow in arrows {
            graph.add_arrow(NodeId(arrow.0), NodeId(arrow.1));
        }

        graph
    }

    fn as_vec(list: Vec<NodeId>) -> Vec<usize> {
        let values : Vec<usize> = list.iter().map(|i| i.0).collect();

        values
    }
}
