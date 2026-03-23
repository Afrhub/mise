use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::topology::{Topology, TopologyGraph};

#[derive(Debug, Error)]
pub enum SwarmError {
    #[error("max_agents must be at least 1, got {0}")]
    InvalidMaxAgents(usize),
    #[error("unknown strategy: {0}")]
    UnknownStrategy(String),
}

/// Strategy for agent assignment within the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Strategy {
    /// Automatically select the best assignment based on topology.
    Auto,
    /// Round-robin assignment across available nodes.
    RoundRobin,
    /// Fill each node before moving to the next.
    Fill,
}

impl Strategy {
    pub fn from_str(s: &str) -> Result<Self, SwarmError> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "round-robin" | "roundrobin" | "round_robin" => Ok(Self::RoundRobin),
            "fill" => Ok(Self::Fill),
            other => Err(SwarmError::UnknownStrategy(other.to_string())),
        }
    }

    /// Resolve `Auto` to a concrete strategy based on the topology.
    pub fn resolve(self, topology: Topology) -> Self {
        if self != Self::Auto {
            return self;
        }
        match topology {
            Topology::Hierarchical => Self::Fill,
            Topology::Mesh => Self::RoundRobin,
            Topology::Ring => Self::RoundRobin,
            Topology::Star => Self::Fill,
        }
    }
}

/// Configuration for initializing a swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmConfig {
    pub topology: Topology,
    pub strategy: Strategy,
    pub max_agents: usize,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            topology: Topology::Hierarchical,
            strategy: Strategy::Auto,
            max_agents: 8,
        }
    }
}

/// An agent node in the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: usize,
    pub role: AgentRole,
}

/// Role assigned to an agent based on its position in the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    Coordinator,
    Worker,
}

/// The initialized swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swarm {
    pub config: SwarmConfig,
    pub agents: Vec<Agent>,
    pub graph: TopologyGraph,
    pub resolved_strategy: Strategy,
}

/// Initialize a swarm with the given configuration.
///
/// This is the main entry point, matching the API:
/// ```ignore
/// swarm_init({ topology: "hierarchical", strategy: "auto", maxAgents: 8 })
/// ```
pub fn swarm_init(config: SwarmConfig) -> Result<Swarm, SwarmError> {
    if config.max_agents == 0 {
        return Err(SwarmError::InvalidMaxAgents(0));
    }

    let resolved_strategy = config.strategy.resolve(config.topology);
    let graph = TopologyGraph::build(config.topology, config.max_agents);

    let agents: Vec<Agent> = (0..config.max_agents)
        .map(|id| {
            let role = assign_role(config.topology, id);
            Agent { id, role }
        })
        .collect();

    Ok(Swarm {
        config,
        agents,
        graph,
        resolved_strategy,
    })
}

/// Assign a role to an agent based on topology and position.
fn assign_role(topology: Topology, id: usize) -> AgentRole {
    match topology {
        Topology::Hierarchical => {
            if id == 0 {
                AgentRole::Coordinator
            } else {
                AgentRole::Worker
            }
        }
        Topology::Star => {
            if id == 0 {
                AgentRole::Coordinator
            } else {
                AgentRole::Worker
            }
        }
        Topology::Ring => {
            // First node acts as coordinator in a ring
            if id == 0 {
                AgentRole::Coordinator
            } else {
                AgentRole::Worker
            }
        }
        Topology::Mesh => {
            // In mesh, node 0 is the initial coordinator but all peers are equal
            if id == 0 {
                AgentRole::Coordinator
            } else {
                AgentRole::Worker
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = SwarmConfig::default();
        assert_eq!(config.topology, Topology::Hierarchical);
        assert_eq!(config.strategy, Strategy::Auto);
        assert_eq!(config.max_agents, 8);
    }

    #[test]
    fn init_hierarchical() {
        let swarm = swarm_init(SwarmConfig {
            topology: Topology::Hierarchical,
            strategy: Strategy::Auto,
            max_agents: 8,
        })
        .unwrap();

        assert_eq!(swarm.agents.len(), 8);
        assert_eq!(swarm.agents[0].role, AgentRole::Coordinator);
        assert_eq!(swarm.resolved_strategy, Strategy::Fill);
        assert_eq!(swarm.graph.node_count, 8);
    }

    #[test]
    fn init_mesh() {
        let swarm = swarm_init(SwarmConfig {
            topology: Topology::Mesh,
            strategy: Strategy::Auto,
            max_agents: 4,
        })
        .unwrap();

        assert_eq!(swarm.agents.len(), 4);
        assert_eq!(swarm.resolved_strategy, Strategy::RoundRobin);
    }

    #[test]
    fn init_ring() {
        let swarm = swarm_init(SwarmConfig {
            topology: Topology::Ring,
            strategy: Strategy::Auto,
            max_agents: 6,
        })
        .unwrap();

        assert_eq!(swarm.agents.len(), 6);
        assert_eq!(swarm.graph.topology, Topology::Ring);
    }

    #[test]
    fn init_star() {
        let swarm = swarm_init(SwarmConfig {
            topology: Topology::Star,
            strategy: Strategy::Auto,
            max_agents: 5,
        })
        .unwrap();

        assert_eq!(swarm.agents.len(), 5);
        assert_eq!(swarm.agents[0].role, AgentRole::Coordinator);
        for i in 1..5 {
            assert_eq!(swarm.agents[i].role, AgentRole::Worker);
        }
    }

    #[test]
    fn zero_agents_errors() {
        let result = swarm_init(SwarmConfig {
            topology: Topology::Star,
            strategy: Strategy::Auto,
            max_agents: 0,
        });
        assert!(result.is_err());
    }

    #[test]
    fn json_roundtrip() {
        let config = SwarmConfig {
            topology: Topology::Mesh,
            strategy: Strategy::RoundRobin,
            max_agents: 4,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SwarmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.topology, Topology::Mesh);
        assert_eq!(parsed.strategy, Strategy::RoundRobin);
        assert_eq!(parsed.max_agents, 4);
    }
}
