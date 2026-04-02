use std::collections::HashMap;

use crate::agent::SpawnedAgent;

use super::runner::ToolRunner;
use super::target::TestTarget;

/// Resolves which runners apply to a given agent + target combination.
pub struct CoverageResolver<'a> {
    runners: &'a HashMap<String, Box<dyn ToolRunner>>,
}

impl<'a> CoverageResolver<'a> {
    pub fn new(runners: &'a HashMap<String, Box<dyn ToolRunner>>) -> Self {
        Self { runners }
    }

    /// For a given target, return runners that match the agent's capabilities
    /// AND support the target's kind.
    pub fn resolve(
        &self,
        agent: &SpawnedAgent,
        target: &TestTarget,
    ) -> Vec<&'a dyn ToolRunner> {
        agent
            .capabilities
            .iter()
            .filter_map(|cap| self.runners.get(&cap.0))
            .filter(|runner| runner.scope().supported_kinds.contains(&target.kind))
            .map(|boxed| boxed.as_ref())
            .collect()
    }

    /// List all capabilities an agent has that have no registered runner.
    pub fn unresolved_capabilities(&self, agent: &SpawnedAgent) -> Vec<String> {
        agent
            .capabilities
            .iter()
            .filter(|cap| !self.runners.contains_key(&cap.0))
            .map(|cap| cap.0.clone())
            .collect()
    }
}
