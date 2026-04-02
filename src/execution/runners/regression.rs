use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Generic regression test runner.
///
/// Looks for a `test` script in the project (npm test, cargo test, pytest, etc.)
/// and runs it. Supports all target kinds as regression tests are universal.
pub struct RegressionRunner;

impl ToolRunner for RegressionRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "regression-testing".into(),
            supported_kinds: vec![
                TargetKind::WebUrl,
                TargetKind::ApiEndpoint,
                TargetKind::MobileApp,
                TargetKind::Service,
            ],
            description: "Generic regression test suite runner".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        // Regression tests use whatever test runner the project has.
        // We check for common ones and succeed if any is found.
        let runners = ["npm", "cargo", "pytest", "mvn", "gradle"];
        for runner in runners {
            if super::super::runner::check_binary_on_path(runner, "regression-testing").is_ok() {
                return Ok(());
            }
        }
        Err(RunnerError::ToolNotInstalled {
            tool: "regression-testing".into(),
            binary: "npm|cargo|pytest|mvn|gradle".into(),
        })
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        // Detect the project type and build the appropriate test command.
        // Priority: cargo > npm > pytest > mvn
        if std::path::Path::new("Cargo.toml").exists() {
            let mut cmd = Command::new("cargo");
            cmd.args(["test", "--", "--format=json", "-Z", "unstable-options"]);
            cmd.env("TEST_TARGET", &target.locator);
            return Ok(cmd);
        }
        if std::path::Path::new("package.json").exists() {
            let mut cmd = Command::new("npm");
            cmd.args(["test", "--", "--reporter=json"]);
            cmd.env("TEST_TARGET", &target.locator);
            return Ok(cmd);
        }
        if std::path::Path::new("pytest.ini").exists()
            || std::path::Path::new("pyproject.toml").exists()
        {
            let mut cmd = Command::new("pytest");
            cmd.args(["--tb=short", "-q", "--json-report"]);
            cmd.env("TEST_TARGET", &target.locator);
            return Ok(cmd);
        }
        // Fallback: npm test
        let mut cmd = Command::new("npm");
        cmd.arg("test");
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
            Some(0) => (Verdict::Pass, "regression suite passed".into()),
            Some(code) => (
                Verdict::Fail,
                format!("regression suite failed (exit code {code})"),
            ),
            None => (
                Verdict::Error,
                "regression test process terminated by signal".into(),
            ),
        };

        TestResult {
            capability: "regression-testing".into(),
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
