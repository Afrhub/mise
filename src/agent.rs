use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentSpawnError {
    #[error("unknown agent type: {0}")]
    UnknownType(String),
    #[error("agent name cannot be empty")]
    EmptyName,
    #[error("agent name already exists: {0}")]
    DuplicateName(String),
    #[error("swarm capacity reached (max {max}), cannot spawn agent \"{name}\"")]
    CapacityReached { max: usize, name: String },
}

/// The type/role of a spawned agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    Architect,
    Coder,
    Tester,
    Reviewer,
    Documenter,
}

impl FromStr for AgentType {
    type Err = AgentSpawnError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "architect" => Ok(Self::Architect),
            "coder" => Ok(Self::Coder),
            "tester" => Ok(Self::Tester),
            "reviewer" => Ok(Self::Reviewer),
            "documenter" => Ok(Self::Documenter),
            other => Err(AgentSpawnError::UnknownType(other.to_string())),
        }
    }
}

impl fmt::Display for AgentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Architect => write!(f, "architect"),
            Self::Coder => write!(f, "coder"),
            Self::Tester => write!(f, "tester"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Documenter => write!(f, "documenter"),
        }
    }
}

/// A capability that an agent possesses.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability(pub String);

impl Capability {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

/// Configuration for spawning a new agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnConfig {
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    pub name: String,
    pub capabilities: Vec<Capability>,
}

/// A spawned agent within the swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedAgent {
    pub id: usize,
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    pub name: String,
    pub capabilities: Vec<Capability>,
    pub status: AgentStatus,
    pub node: usize,
}

impl SpawnedAgent {
    /// Check if this agent has a given capability.
    pub fn has_capability(&self, name: &str) -> bool {
        self.capabilities.iter().any(|c| c.0 == name)
    }
}

/// Runtime status of a spawned agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Idle,
    Running,
    Stopped,
}

/// Registry that tracks all spawned agents within a swarm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistry {
    agents: Vec<SpawnedAgent>,
    max_agents: usize,
    next_node: usize,
    node_count: usize,
}

impl AgentRegistry {
    pub fn new(max_agents: usize, node_count: usize) -> Self {
        Self {
            agents: Vec::new(),
            max_agents,
            next_node: 0,
            node_count: node_count.max(1),
        }
    }

    /// Spawn a new agent and assign it to a topology node via round-robin.
    ///
    /// Returns an error if:
    /// - `name` is empty (`AgentSpawnError::EmptyName`)
    /// - `name` is already taken (`AgentSpawnError::DuplicateName`)
    /// - registry is at capacity (`AgentSpawnError::CapacityReached`)
    pub fn spawn(&mut self, config: SpawnConfig) -> Result<&SpawnedAgent, AgentSpawnError> {
        if config.name.is_empty() {
            return Err(AgentSpawnError::EmptyName);
        }
        if self.agents.iter().any(|a| a.name == config.name) {
            return Err(AgentSpawnError::DuplicateName(config.name));
        }
        if self.agents.len() >= self.max_agents {
            return Err(AgentSpawnError::CapacityReached {
                max: self.max_agents,
                name: config.name,
            });
        }

        let id = self.agents.len();
        let node = self.next_node % self.node_count;
        self.next_node += 1;

        let agent = SpawnedAgent {
            id,
            agent_type: config.agent_type,
            name: config.name,
            capabilities: config.capabilities,
            status: AgentStatus::Idle,
            node,
        };

        self.agents.push(agent);
        // Safety: we just pushed, so last() is guaranteed Some
        Ok(self.agents.last().expect("just pushed"))
    }

    pub fn agents(&self) -> &[SpawnedAgent] {
        &self.agents
    }

    pub fn get_by_name(&self, name: &str) -> Option<&SpawnedAgent> {
        self.agents.iter().find(|a| a.name == name)
    }

    pub fn get_by_type(&self, agent_type: AgentType) -> Vec<&SpawnedAgent> {
        self.agents.iter().filter(|a| a.agent_type == agent_type).collect()
    }

    pub fn count(&self) -> usize {
        self.agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config(agent_type: &str, name: &str, caps: &[&str]) -> SpawnConfig {
        SpawnConfig {
            agent_type: agent_type.parse().unwrap(),
            name: name.to_string(),
            capabilities: caps.iter().map(|c| Capability::new(c)).collect(),
        }
    }

    #[test]
    fn spawn_single_agent() {
        let mut reg = AgentRegistry::new(8, 8);
        let agent = reg
            .spawn(make_config("architect", "system-designer", &["database-schema", "api-design", "security-patterns"]))
            .unwrap();

        assert_eq!(agent.id, 0);
        assert_eq!(agent.agent_type, AgentType::Architect);
        assert_eq!(agent.name, "system-designer");
        assert_eq!(agent.capabilities.len(), 3);
        assert_eq!(agent.status, AgentStatus::Idle);
        assert_eq!(agent.node, 0);
    }

    #[test]
    fn spawn_all_five_agent_types() {
        let mut reg = AgentRegistry::new(8, 8);

        reg.spawn(make_config("architect", "system-designer", &["database-schema", "api-design", "security-patterns"])).unwrap();
        reg.spawn(make_config("coder", "feature-builder", &["react-native", "typescript", "nativewind"])).unwrap();
        reg.spawn(make_config("tester", "qa-agent", &["regression-testing", "security-audit", "edge-cases", "playwright", "karate", "appium", "docker-selenium", "qa-wolf", "mabl"])).unwrap();
        reg.spawn(make_config("reviewer", "code-reviewer", &["code-quality", "best-practices", "performance"])).unwrap();
        reg.spawn(make_config("documenter", "docs-agent", &["api-docs", "user-guides", "changelog"])).unwrap();

        assert_eq!(reg.count(), 5);
        assert_eq!(reg.get_by_name("system-designer").unwrap().agent_type, AgentType::Architect);
        assert_eq!(reg.get_by_name("feature-builder").unwrap().agent_type, AgentType::Coder);
        assert_eq!(reg.get_by_name("qa-agent").unwrap().agent_type, AgentType::Tester);
        assert_eq!(reg.get_by_name("code-reviewer").unwrap().agent_type, AgentType::Reviewer);
        assert_eq!(reg.get_by_name("docs-agent").unwrap().agent_type, AgentType::Documenter);
    }

    #[test]
    fn round_robin_node_assignment() {
        let mut reg = AgentRegistry::new(8, 4);
        for i in 0..6 {
            reg.spawn(make_config("coder", &format!("agent-{i}"), &["ts"])).unwrap();
        }
        assert_eq!(reg.agents()[0].node, 0);
        assert_eq!(reg.agents()[1].node, 1);
        assert_eq!(reg.agents()[2].node, 2);
        assert_eq!(reg.agents()[3].node, 3);
        assert_eq!(reg.agents()[4].node, 0);
        assert_eq!(reg.agents()[5].node, 1);
    }

    #[test]
    fn duplicate_name_rejected() {
        let mut reg = AgentRegistry::new(8, 8);
        reg.spawn(make_config("coder", "bob", &[])).unwrap();
        let err = reg.spawn(make_config("tester", "bob", &[])).unwrap_err();
        assert!(matches!(err, AgentSpawnError::DuplicateName(_)));
    }

    #[test]
    fn empty_name_rejected() {
        let mut reg = AgentRegistry::new(8, 8);
        let err = reg.spawn(make_config("coder", "", &[])).unwrap_err();
        assert!(matches!(err, AgentSpawnError::EmptyName));
    }

    #[test]
    fn capacity_enforced() {
        let mut reg = AgentRegistry::new(2, 4);
        reg.spawn(make_config("coder", "a", &[])).unwrap();
        reg.spawn(make_config("coder", "b", &[])).unwrap();
        let err = reg.spawn(make_config("coder", "c", &[])).unwrap_err();
        assert!(matches!(err, AgentSpawnError::CapacityReached { .. }));
    }

    #[test]
    fn get_by_type() {
        let mut reg = AgentRegistry::new(8, 8);
        reg.spawn(make_config("coder", "a", &[])).unwrap();
        reg.spawn(make_config("tester", "b", &[])).unwrap();
        reg.spawn(make_config("coder", "c", &[])).unwrap();
        assert_eq!(reg.get_by_type(AgentType::Coder).len(), 2);
        assert_eq!(reg.get_by_type(AgentType::Tester).len(), 1);
        assert_eq!(reg.get_by_type(AgentType::Architect).len(), 0);
    }

    #[test]
    fn get_by_name_not_found() {
        let reg = AgentRegistry::new(8, 8);
        assert!(reg.get_by_name("nonexistent").is_none());
    }

    #[test]
    fn json_roundtrip() {
        let config = make_config("architect", "sys", &["api-design", "security"]);
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SpawnConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.agent_type, AgentType::Architect);
        assert_eq!(parsed.name, "sys");
        assert_eq!(parsed.capabilities.len(), 2);
    }

    #[test]
    fn unknown_type_rejected() {
        let err = "hacker".parse::<AgentType>().unwrap_err();
        assert!(matches!(err, AgentSpawnError::UnknownType(_)));
    }

    #[test]
    fn agent_type_display() {
        assert_eq!(AgentType::Architect.to_string(), "architect");
        assert_eq!(AgentType::Coder.to_string(), "coder");
    }
}
