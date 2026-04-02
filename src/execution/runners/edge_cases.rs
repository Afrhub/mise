use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Edge-case and boundary testing runner.
///
/// Runs fuzz/boundary tests using the project's test framework
/// with edge-case-specific test tags or markers.
pub struct EdgeCasesRunner;

impl ToolRunner for EdgeCasesRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "edge-cases".into(),
            supported_kinds: vec![
                TargetKind::WebUrl,
                TargetKind::ApiEndpoint,
                TargetKind::MobileApp,
                TargetKind::Service,
            ],
            description: "Edge-case and boundary condition testing".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        let runners = ["npm", "cargo", "pytest"];
        for runner in runners {
            if super::super::runner::check_binary_on_path(runner, "edge-cases").is_ok() {
                return Ok(());
            }
        }
        Err(RunnerError::ToolNotInstalled {
            tool: "edge-cases".into(),
            binary: "npm|cargo|pytest".into(),
        })
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        if std::path::Path::new("Cargo.toml").exists() {
            let mut cmd = Command::new("cargo");
            // Run only tests tagged with "edge" or "boundary"
            cmd.args(["test", "edge", "--", "--format=json", "-Z", "unstable-options"]);
            cmd.env("TEST_TARGET", &target.locator);
            return Ok(cmd);
        }
        if std::path::Path::new("package.json").exists() {
            let mut cmd = Command::new("npm");
            cmd.args(["test", "--", "--grep", "edge|boundary"]);
            cmd.env("TEST_TARGET", &target.locator);
            return Ok(cmd);
        }
        // pytest with markers
        let mut cmd = Command::new("pytest");
        cmd.args(["-m", "edge or boundary", "--tb=short", "-q"]);
        cmd.env("TEST_TARGET", &target.locator);
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
            Some(0) => (Verdict::Pass, "edge-case tests passed".into()),
            Some(code) => (
                Verdict::Fail,
                format!("edge-case tests failed (exit code {code})"),
            ),
            None => (
                Verdict::Error,
                "edge-case test process terminated by signal".into(),
            ),
        };

        TestResult {
            capability: "edge-cases".into(),
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
