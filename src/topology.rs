use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Error)]
#[error("unknown topology: {0}")]
pub struct ParseTopologyError(String);

/// Supported swarm topologies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Topology {
    Hierarchical,
    Mesh,
    Ring,
    Star,
}

impl FromStr for Topology {
    type Err = ParseTopologyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hierarchical" => Ok(Self::Hierarchical),
            "mesh" => Ok(Self::Mesh),
            "ring" => Ok(Self::Ring),
            "star" => Ok(Self::Star),
            other => Err(ParseTopologyError(other.to_string())),
        }
    }
}

impl fmt::Display for Topology {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hierarchical => write!(f, "hierarchical"),
            Self::Mesh => write!(f, "mesh"),
            Self::Ring => write!(f, "ring"),
            Self::Star => write!(f, "star"),
        }
    }
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
    pub adjacency: BTreeMap<usize, Vec<usize>>,
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
fn build_hierarchical(n: usize) -> (Vec<Edge>, BTreeMap<usize, Vec<usize>>) {
    let mut edges = Vec::with_capacity(2 * n.saturating_sub(1));
    let mut adj: BTreeMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::with_capacity(3))).collect();

    for i in 0..n {
        let left = 2 * i + 1;
        let right = 2 * i + 2;
        if left < n {
            edges.push(Edge { from: i, to: left });
            edges.push(Edge { from: left, to: i });
            adj.entry(i).or_default().push(left);
            adj.entry(left).or_default().push(i);
        }
        if right < n {
            edges.push(Edge { from: i, to: right });
            edges.push(Edge { from: right, to: i });
            adj.entry(i).or_default().push(right);
            adj.entry(right).or_default().push(i);
        }
    }
    (edges, adj)
}

/// Mesh (fully connected): every node connects to every other node.
fn build_mesh(n: usize) -> (Vec<Edge>, BTreeMap<usize, Vec<usize>>) {
    let edge_count = n * n.saturating_sub(1);
    let mut edges = Vec::with_capacity(edge_count);
    let mut adj: BTreeMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::with_capacity(n.saturating_sub(1)))).collect();

    for i in 0..n {
        for j in (i + 1)..n {
            edges.push(Edge { from: i, to: j });
            edges.push(Edge { from: j, to: i });
            adj.entry(i).or_default().push(j);
            adj.entry(j).or_default().push(i);
        }
    }
    (edges, adj)
}

/// Ring: each node connects to its next neighbor, forming a cycle.
fn build_ring(n: usize) -> (Vec<Edge>, BTreeMap<usize, Vec<usize>>) {
    let mut edges = Vec::with_capacity(2 * n);
    let mut adj: BTreeMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::with_capacity(2))).collect();

    if n < 2 {
        return (edges, adj);
    }

    for i in 0..n {
        let next = (i + 1) % n;
        // Add forward and reverse edges; each pair appears exactly once
        edges.push(Edge { from: i, to: next });
        edges.push(Edge { from: next, to: i });
        adj.entry(i).or_default().push(next);
        adj.entry(next).or_default().push(i);
    }

    // Deduplicate adjacency lists (ring loop visits each pair twice for the wrap-around)
    for neighbors in adj.values_mut() {
        neighbors.sort_unstable();
        neighbors.dedup();
    }

    // Deduplicate edges
    edges.sort_by(|a, b| (a.from, a.to).cmp(&(b.from, b.to)));
    edges.dedup();

    (edges, adj)
}

/// Star: node 0 is the hub, all other nodes connect only to node 0.
fn build_star(n: usize) -> (Vec<Edge>, BTreeMap<usize, Vec<usize>>) {
    let mut edges = Vec::with_capacity(2 * n.saturating_sub(1));
    let mut adj: BTreeMap<usize, Vec<usize>> = (0..n).map(|i| (i, Vec::with_capacity(if i == 0 { n.saturating_sub(1) } else { 1 }))).collect();

    for i in 1..n {
        edges.push(Edge { from: 0, to: i });
        edges.push(Edge { from: i, to: 0 });
        adj.entry(0).or_default().push(i);
        adj.entry(i).or_default().push(0);
    }
    (edges, adj)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_has_correct_edge_count() {
        let graph = TopologyGraph::build(Topology::Star, 5);
        assert_eq!(graph.edges.len(), 8);
        assert_eq!(graph.adjacency[&0].len(), 4);
        for i in 1..5 {
            assert_eq!(graph.adjacency[&i].len(), 1);
        }
    }

    #[test]
    fn ring_forms_cycle() {
        let graph = TopologyGraph::build(Topology::Ring, 4);
        assert_eq!(graph.edges.len(), 8);
        for i in 0..4 {
            assert_eq!(graph.adjacency[&i].len(), 2);
        }
    }

    #[test]
    fn mesh_is_fully_connected() {
        let graph = TopologyGraph::build(Topology::Mesh, 4);
        assert_eq!(graph.edges.len(), 12);
        for i in 0..4 {
            assert_eq!(graph.adjacency[&i].len(), 3);
        }
    }

    #[test]
    fn hierarchical_is_binary_tree() {
        let graph = TopologyGraph::build(Topology::Hierarchical, 7);
        assert_eq!(graph.edges.len(), 12);
        assert_eq!(graph.adjacency[&0].len(), 2);
        assert_eq!(graph.adjacency[&1].len(), 3);
        assert_eq!(graph.adjacency[&2].len(), 3);
        for i in 3..7 {
            assert_eq!(graph.adjacency[&i].len(), 1);
        }
    }

    // Edge-case tests for n=0 and n=1
    #[test]
    fn all_topologies_n0() {
        for topo in [Topology::Hierarchical, Topology::Mesh, Topology::Ring, Topology::Star] {
            let graph = TopologyGraph::build(topo, 0);
            assert_eq!(graph.node_count, 0);
            assert!(graph.edges.is_empty());
            assert!(graph.adjacency.is_empty());
        }
    }

    #[test]
    fn all_topologies_n1() {
        for topo in [Topology::Hierarchical, Topology::Mesh, Topology::Ring, Topology::Star] {
            let graph = TopologyGraph::build(topo, 1);
            assert_eq!(graph.node_count, 1);
            assert!(graph.edges.is_empty(), "topo {topo}: expected no edges for single node");
            assert_eq!(graph.adjacency[&0].len(), 0);
        }
    }

    #[test]
    fn all_topologies_n2() {
        for topo in [Topology::Hierarchical, Topology::Mesh, Topology::Ring, Topology::Star] {
            let graph = TopologyGraph::build(topo, 2);
            assert_eq!(graph.node_count, 2);
            // All topologies with 2 nodes should have exactly 1 undirected edge = 2 directed
            assert_eq!(graph.edges.len(), 2, "topo {topo}: expected 2 directed edges for 2 nodes");
        }
    }

    #[test]
    fn topology_from_str() {
        assert_eq!("hierarchical".parse::<Topology>().unwrap(), Topology::Hierarchical);
        assert_eq!("MESH".parse::<Topology>().unwrap(), Topology::Mesh);
        assert_eq!("Ring".parse::<Topology>().unwrap(), Topology::Ring);
        assert_eq!("star".parse::<Topology>().unwrap(), Topology::Star);
        assert!("invalid".parse::<Topology>().is_err());
    }

    #[test]
    fn topology_display() {
        assert_eq!(Topology::Hierarchical.to_string(), "hierarchical");
        assert_eq!(Topology::Mesh.to_string(), "mesh");
    }

    #[test]
    fn deterministic_adjacency_order() {
        let g1 = TopologyGraph::build(Topology::Star, 5);
        let g2 = TopologyGraph::build(Topology::Star, 5);
        let j1 = serde_json::to_string(&g1.adjacency).unwrap();
        let j2 = serde_json::to_string(&g2.adjacency).unwrap();
        assert_eq!(j1, j2, "BTreeMap should produce deterministic JSON");
    }
}
