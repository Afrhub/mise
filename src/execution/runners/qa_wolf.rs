use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// End-to-end browser testing via QA Wolf.
pub struct QaWolfRunner;

impl ToolRunner for QaWolfRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "qa-wolf".into(),
            supported_kinds: vec![TargetKind::WebUrl],
            description: "End-to-end browser testing via QA Wolf".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("npx", "qa-wolf")
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        let mut cmd = Command::new("npx");
        cmd.args(["qawolf", "test", "--all-browsers"]);
        cmd.env("QAW_BASE_URL", &target.locator);
        for (k, v) in &target.env {
            cmd.env(k, v);
        }
        Ok(cmd)
    }

    fn parse_output(
        &self,
        _target: &TestTarget,
        agent_name: &str,
        output: &std::process::Output,
        duration_ms: u64,
    ) -> TestResult {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code();

        let (verdict, summary) = match exit_code {
            Some(0) => (Verdict::Pass, "all QA Wolf E2E tests passed".into()),
            Some(code) => (Verdict::Fail, format!("qa-wolf exited with code {code}")),
            None => (Verdict::Error, "qa-wolf process terminated by signal".into()),
        };

        TestResult {
            capability: "qa-wolf".into(),
            agent_name: agent_name.to_string(),
            verdict,
            summary,
            stdout,
            stderr,
            exit_code,
            duration_ms,
            coverage_pct: None,
        }
    }
}
