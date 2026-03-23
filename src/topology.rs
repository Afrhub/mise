use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported swarm topologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Topology {
    Hierarchical,
    Mesh,
    Ring,
    Star,
}

/// A directed edge between two agent nodes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
}

/// The computed topology graph for a swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub topology: Topology,
    pub node_count: usize,
    pub edges: Vec<Edge>,
    /// Adjacency list: node index -> list of connected node indices.
    pub adjacency: HashMap<usize, Vec<usize>>,
}

impl TopologyGraph {
    /// Build a topology graph for `n` nodes using the given topology type.
    pub fn build(topology: Topology, n: usize) -> Self {
        let (edges, adjacency) = match topology {
            Topology::Hierarchical => build_hierarchical(n),
            Topology::Mesh => build_mesh(n),
            Topology::Ring => build_ring(n),
            Topology::Star => build_star(n),
        };
        Self {
            topology,
            node_count: n,
            edges,
            adjacency,
        }
    }
}

/// Hierarchical (binary tree): node 0 is root, children of node i are 2i+1 and 2i+2.
fn build_hierarchical(n: usize) -> (Vec<Edge>, HashMap<usize, Vec<usize>>) {
    let mut edges = Vec::new();
    let mut adj: HashMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::new())).collect();

    for i in 0..n {
        let left = 2 * i + 1;
        let right = 2 * i + 2;
        if left < n {
            edges.push(Edge { from: i, to: left });
            edges.push(Edge { from: left, to: i });
            adj.get_mut(&i).unwrap().push(left);
            adj.get_mut(&left).unwrap().push(i);
        }
        if right < n {
            edges.push(Edge { from: i, to: right });
            edges.push(Edge { from: right, to: i });
            adj.get_mut(&i).unwrap().push(right);
            adj.get_mut(&right).unwrap().push(i);
        }
    }
    (edges, adj)
}

/// Mesh (fully connected): every node connects to every other node.
fn build_mesh(n: usize) -> (Vec<Edge>, HashMap<usize, Vec<usize>>) {
    let mut edges = Vec::new();
    let mut adj: HashMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::new())).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            edges.push(Edge { from: i, to: j });
            edges.push(Edge { from: j, to: i });
            adj.get_mut(&i).unwrap().push(j);
            adj.get_mut(&j).unwrap().push(i);
        }
    }
    (edges, adj)
}

/// Ring: each node connects to its next neighbor, forming a cycle.
fn build_ring(n: usize) -> (Vec<Edge>, HashMap<usize, Vec<usize>>) {
    let mut edges = Vec::new();
    let mut adj: HashMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::new())).collect();

    if n < 2 {
        return (edges, adj);
    }

    for i in 0..n {
        let next = (i + 1) % n;
        edges.push(Edge { from: i, to: next });
        edges.push(Edge { from: next, to: i });
        adj.get_mut(&i).unwrap().push(next);
        adj.get_mut(&next).unwrap().push(i);
    }
    (edges, adj)
}

/// Star: node 0 is the hub, all other nodes connect only to node 0.
fn build_star(n: usize) -> (Vec<Edge>, HashMap<usize, Vec<usize>>) {
    let mut edges = Vec::new();
    let mut adj: HashMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::new())).collect();

    for i in 1..n {
        edges.push(Edge { from: 0, to: i });
        edges.push(Edge { from: i, to: 0 });
        adj.get_mut(&0).unwrap().push(i);
        adj.get_mut(&i).unwrap().push(0);
    }
    (edges, adj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_has_correct_edge_count() {
        let graph = TopologyGraph::build(Topology::Star, 5);
        // Star with 5 nodes: 4 spokes * 2 directions = 8 edges
        assert_eq!(graph.edges.len(), 8);
        assert_eq!(graph.adjacency[&0].len(), 4);
        for i in 1..5 {
            assert_eq!(graph.adjacency[&i].len(), 1);
        }
    }

    #[test]
    fn ring_forms_cycle() {
        let graph = TopologyGraph::build(Topology::Ring, 4);
        // Ring with 4 nodes: 4 edges * 2 directions = 8 edges
        assert_eq!(graph.edges.len(), 8);
        for i in 0..4 {
            assert_eq!(graph.adjacency[&i].len(), 2);
        }
    }

    #[test]
    fn mesh_is_fully_connected() {
        let graph = TopologyGraph::build(Topology::Mesh, 4);
        // 4 nodes fully connected: C(4,2) = 6 undirected edges * 2 = 12 directed
        assert_eq!(graph.edges.len(), 12);
        for i in 0..4 {
            assert_eq!(graph.adjacency[&i].len(), 3);
        }
    }

    #[test]
    fn hierarchical_is_binary_tree() {
        let graph = TopologyGraph::build(Topology::Hierarchical, 7);
        // Full binary tree with 7 nodes: 6 undirected edges * 2 = 12 directed
        assert_eq!(graph.edges.len(), 12);
        // Root has 2 children
        assert_eq!(graph.adjacency[&0].len(), 2);
        // Internal nodes have 3 connections (parent + 2 children)
        assert_eq!(graph.adjacency[&1].len(), 3);
        assert_eq!(graph.adjacency[&2].len(), 3);
        // Leaves have 1 connection (parent only)
        for i in 3..7 {
            assert_eq!(graph.adjacency[&i].len(), 1);
        }
    }
}
