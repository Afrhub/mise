use std::process::{Command, Output};
use std::time::Instant;
use thiserror::Error;

use super::result::{TestResult, Verdict};
use super::target::{TargetKind, TestTarget};

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("tool not installed: {tool} (looked for `{binary}` on PATH)")]
    ToolNotInstalled { tool: String, binary: String },
    #[error("subprocess failed: {0}")]
    SubprocessFailed(#[from] std::io::Error),
    #[error("unsupported target kind '{kind}' for runner '{runner}'")]
    UnsupportedTarget { runner: String, kind: String },
}

/// Describes what a runner handles.
#[derive(Debug, Clone)]
pub struct RunnerScope {
    /// The capability string this runner handles (e.g. "playwright").
    pub capability: String,
    /// Which target kinds this runner supports.
    pub supported_kinds: Vec<TargetKind>,
    /// Human-readable description.
    pub description: String,
}

/// The core trait for all tool runners.
///
/// Implementors define their scope, how to check tool availability,
/// how to build the subprocess command, and how to parse results.
pub trait ToolRunner: Send + Sync {
    /// Describe this runner's scope.
    fn scope(&self) -> RunnerScope;

    /// Check whether the external tool is available on the system.
    fn check_available(&self) -> Result<(), RunnerError>;

    /// Build the command and arguments for the subprocess.
    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError>;

    /// Parse subprocess output into a TestResult.
    fn parse_output(
        &self,
        target: &TestTarget,
        agent_name: &str,
        output: &Output,
        duration_ms: u64,
    ) -> TestResult;

    /// Execute the tool against the target.
    ///
    /// Default implementation: check availability, build command, run, parse.
    fn run(&self, target: &TestTarget, agent_name: &str) -> Result<TestResult, RunnerError> {
        if let Err(e) = self.check_available() {
            return Ok(TestResult {
                capability: self.scope().capability.clone(),
                agent_name: agent_name.to_string(),
                verdict: Verdict::Error,
                summary: format!("{e}"),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 0,
                coverage_pct: None,
            });
        }

        if !self.scope().supported_kinds.contains(&target.kind) {
            return Ok(TestResult {
                capability: self.scope().capability.clone(),
                agent_name: agent_name.to_string(),
                verdict: Verdict::Skip,
                summary: format!(
                    "runner '{}' does not support target kind '{}'",
                    self.scope().capability,
                    target.kind
                ),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: None,
                duration_ms: 0,
                coverage_pct: None,
            });
        }

        let mut cmd = self.build_command(target)?;
        let start = Instant::now();
        let output = cmd.output()?;
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(self.parse_output(target, agent_name, &output, duration_ms))
    }
}

/// Check if a binary is available on PATH.
pub fn check_binary_on_path(binary: &str, tool_name: &str) -> Result<(), RunnerError> {
    match Command::new("which").arg(binary).output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(RunnerError::ToolNotInstalled {
            tool: tool_name.to_string(),
            binary: binary.to_string(),
        }),
    }
}
