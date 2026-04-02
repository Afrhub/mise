pub mod coverage;
pub mod result;
pub mod runner;
pub mod runners;
pub mod target;

use std::collections::HashMap;

use crate::agent::SpawnedAgent;
use crate::swarm::Swarm;

use coverage::CoverageResolver;
use result::{AggregatedResults, TestResult, Verdict};
use runner::ToolRunner;
use target::TestTarget;

/// The central dispatcher that maps agent capabilities to tool runners
/// and orchestrates test execution.
pub struct CapabilityDispatcher {
    runners: HashMap<String, Box<dyn ToolRunner>>,
}

impl CapabilityDispatcher {
    /// Create a dispatcher with all built-in runners registered.
    pub fn new() -> Self {
        Self {
            runners: runners::default_runners(),
        }
    }

    /// Execute all applicable capabilities for a single agent against a target.
    ///
    /// Returns a TestResult for every capability the agent has:
    /// - Pass/Fail if the runner executed
    /// - Skip if the runner doesn't support the target kind
    /// - Error if the tool isn't installed or failed to run
    pub fn execute_for_agent(
        &self,
        agent: &SpawnedAgent,
        target: &TestTarget,
    ) -> Vec<TestResult> {
        let resolver = CoverageResolver::new(&self.runners);

        // First, produce results for capabilities that have matching runners
        let mut results: Vec<TestResult> = resolver
            .resolve(agent, target)
            .into_iter()
            .map(|runner| match runner.run(target, &agent.name) {
                Ok(result) => result,
                Err(e) => TestResult {
                    capability: runner.scope().capability.clone(),
                    agent_name: agent.name.clone(),
                    verdict: Verdict::Error,
                    summary: format!("{e}"),
                    stdout: String::new(),
                    stderr: String::new(),
                    exit_code: None,
                    duration_ms: 0,
                    coverage_pct: None,
                },
            })
            .collect();

        // Report unresolved capabilities as skipped
        for cap_name in resolver.unresolved_capabilities(agent) {
            results.push(TestResult {
                capability: cap_name.clone(),
                agent_name: agent.name.clone(),
                verdict: Verdict::Skip,
                summary: format!("no runner registered for capability '{cap_name}'"),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 0,
                coverage_pct: None,
            });
        }

        results
    }

    /// Execute across all agents in a swarm and aggregate results.
    pub fn execute_swarm(
        &self,
        swarm: &Swarm,
        target: &TestTarget,
    ) -> AggregatedResults {
        let all_results: Vec<TestResult> = swarm
            .registry
            .agents()
            .iter()
            .flat_map(|agent| self.execute_for_agent(agent, target))
            .collect();

        AggregatedResults::aggregate(target.clone(), all_results)
    }
}

impl Default for CapabilityDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
