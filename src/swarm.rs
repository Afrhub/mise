use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

use crate::agent::{AgentRegistry, AgentSpawnError, SpawnConfig, SpawnedAgent};
use crate::execution::result::AggregatedResults;
use crate::execution::target::TestTarget;
use crate::execution::CapabilityDispatcher;
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

impl FromStr for Strategy {
    type Err = SwarmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "round-robin" | "roundrobin" | "round_robin" => Ok(Self::RoundRobin),
            "fill" => Ok(Self::Fill),
            other => Err(SwarmError::UnknownStrategy(other.to_string())),
        }
    }
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auto => write!(f, "auto"),
            Self::RoundRobin => write!(f, "round-robin"),
            Self::Fill => write!(f, "fill"),
        }
    }
}

impl Strategy {
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

/// A topology node in the swarm (distinct from a spawned agent).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: usize,
    pub role: NodeRole,
}

/// Role assigned to a node based on its position in the topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Coordinator,
    Worker,
}

/// The initialized swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swarm {
    pub config: SwarmConfig,
    /// Topology nodes. Spawned agents are assigned to these nodes via the registry.
    pub nodes: Vec<Node>,
    pub graph: TopologyGraph,
    pub resolved_strategy: Strategy,
    pub registry: AgentRegistry,
}

impl Swarm {
    /// Spawn a typed agent into this swarm's registry.
    ///
    /// Returns an error if the name is empty, duplicate, or the swarm is at capacity.
    /// Agents are assigned to topology nodes via round-robin.
    pub fn spawn_agent(&mut self, config: SpawnConfig) -> Result<&SpawnedAgent, AgentSpawnError> {
        self.registry.spawn(config)
    }

    /// Execute all applicable capabilities across all spawned agents for a target.
    ///
    /// Each agent's capabilities are matched against registered tool runners.
    /// Only runners that support the target's kind are executed.
    pub fn execute(&self, target: &TestTarget) -> AggregatedResults {
        let dispatcher = CapabilityDispatcher::new();
        dispatcher.execute_swarm(self, target)
    }
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

    let nodes: Vec<Node> = (0..config.max_agents)
        .map(|id| {
            let role = assign_role(config.topology, id);
            Node { id, role }
        })
        .collect();

    let registry = AgentRegistry::new(config.max_agents, config.max_agents);

    Ok(Swarm {
        config,
        nodes,
        graph,
        resolved_strategy,
        registry,
    })
}

/// Assign a role to a node based on topology and position.
fn assign_role(topology: Topology, id: usize) -> NodeRole {
    if id == 0 {
        NodeRole::Coordinator
    } else {
        match topology {
            // In all topologies, node 0 is the coordinator
            Topology::Hierarchical
            | Topology::Star
            | Topology::Ring
            | Topology::Mesh => NodeRole::Worker,
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

        assert_eq!(swarm.nodes.len(), 8);
        assert_eq!(swarm.nodes[0].role, NodeRole::Coordinator);
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

        assert_eq!(swarm.nodes.len(), 4);
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

        assert_eq!(swarm.nodes.len(), 6);
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

        assert_eq!(swarm.nodes.len(), 5);
        assert_eq!(swarm.nodes[0].role, NodeRole::Coordinator);
        for i in 1..5 {
            assert_eq!(swarm.nodes[i].role, NodeRole::Worker);
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

    #[test]
    fn strategy_from_str() {
        assert_eq!("auto".parse::<Strategy>().unwrap(), Strategy::Auto);
        assert_eq!("round-robin".parse::<Strategy>().unwrap(), Strategy::RoundRobin);
        assert_eq!("roundrobin".parse::<Strategy>().unwrap(), Strategy::RoundRobin);
        assert_eq!("round_robin".parse::<Strategy>().unwrap(), Strategy::RoundRobin);
        assert_eq!("fill".parse::<Strategy>().unwrap(), Strategy::Fill);
        assert!("invalid".parse::<Strategy>().is_err());
    }

    #[test]
    fn strategy_resolve_non_auto_unchanged() {
        assert_eq!(Strategy::Fill.resolve(Topology::Mesh), Strategy::Fill);
        assert_eq!(Strategy::RoundRobin.resolve(Topology::Star), Strategy::RoundRobin);
    }

    #[test]
    fn spawn_agent_through_swarm() {
        use crate::agent::{AgentType, Capability, SpawnConfig};

        let mut swarm = swarm_init(SwarmConfig::default()).unwrap();
        let agent = swarm.spawn_agent(SpawnConfig {
            agent_type: AgentType::Architect,
            name: "test-arch".to_string(),
            capabilities: vec![Capability::new("api-design")],
        }).unwrap();

        assert_eq!(agent.name, "test-arch");
        assert_eq!(swarm.registry.count(), 1);
    }
}
