use fixedbitset::FixedBitSet;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct NodeId(pub(crate) usize);

#[derive(Clone)]
pub struct Preorder {
    nodes: Vec<Node>,
}

#[derive(Clone)]
struct Node {
    id: NodeId,

    weight: u64, // greedy value

    incoming: Vec<NodeId>,
    outgoing: Vec<NodeId>,
}

impl Preorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, weight: u64) -> NodeId {
        let id = NodeId(self.nodes.len());

        self.nodes.push(Node::new(id, weight));

        id
    }

    pub fn add_arrow(&mut self, source_id: NodeId, target_id: NodeId) {
        assert_ne!(source_id, target_id);

        self.nodes[source_id.0].outgoing.push(target_id);
        self.nodes[target_id.0].incoming.push(source_id);

        // println!("  Arrow {:?} {:?}", source_id, target_id);
    }

    pub fn sort(&self) -> Vec<NodeId> {
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
                    results.push(node.id);
                    //println!("   Item: {:?}", node.id());
                }
            }

            if results.len() == start_len {
                panic!("preorder sort unable to make progress, possibly due to loops");
            }

            let new_results = &mut results.as_mut_slice()[start_len..];
            
            new_results.sort_by_key(|n| u64::MAX - self.nodes[n.0].weight);

            pending.difference_with(&completed);
        }

        assert!(results.len() == self.nodes.len());
        results
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
}

#[cfg(test)]
mod tests {
    use super::{Preorder, NodeId};

    #[test]
    fn empty() {
        let graph = Preorder::new();

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
        let g = graph(2, &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1]);

        let g = graph(2, &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0]);
    }

    #[test]
    fn triple() {
        // single arrows
        let g = graph(3, &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        let g = graph(3, &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let g = graph(3, &[(0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let g = graph(3, &[(2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let g = graph(3, &[(1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let g = graph(3, &[(2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        // two arrows
        let g = graph(3, &[(0, 1), (0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let g = graph(3, &[(0, 1), (2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);

        let g = graph(3, &[(1, 0), (2, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let g = graph(3, &[(1, 0), (0, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0, 2]);

        // --
        let g = graph(3, &[(0, 1), (1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1, 2]);

        let g = graph(3, &[(1, 0), (1, 2)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0, 2]);

        let g = graph(3, &[(1, 0), (2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let g = graph(3, &[(0, 1), (2, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);
    }

    #[test]
    #[should_panic]
    fn cycle() {
        let g = graph(3, &[(0, 1), (1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);
    }

    #[test]
    fn weights_no_arrows() {
        let g = graph_w(&[0, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 0]);

        let g = graph_w(&[1, 0], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 1]);

        let g = graph_w(&[0, 1, 2], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let g = graph_w(&[0, 2, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);

        let g = graph_w(&[0, 0, 1], &[]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);
    }

    #[test]
    fn weights_triple_one_arrow() {
        let g = graph_w(&[0, 1, 2], &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 0, 1]);

        let g = graph_w(&[0, 1, 2], &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [2, 1, 0]);

        let g = graph_w(&[2, 1, 0], &[(0, 1)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [0, 2, 1]);

        let g = graph_w(&[2, 1, 0], &[(1, 0)]);
        assert_eq!(as_vec(g.sort()).as_slice(), [1, 2, 0]);
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
