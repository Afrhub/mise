use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Mobile app testing via Appium.
pub struct AppiumRunner;

impl ToolRunner for AppiumRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "appium".into(),
            supported_kinds: vec![TargetKind::MobileApp],
            description: "Mobile application testing via Appium server".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("appium", "appium")
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        // Appium tests are typically driven by a test framework (e.g. pytest, mocha)
        // that connects to an Appium server. We invoke the test runner.
        let mut cmd = Command::new("npx");
        cmd.args(["wdio", "run", "wdio.conf.js", "--reporter=json"]);
        cmd.env("APP_PATH", &target.locator);
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
            Some(0) => (Verdict::Pass, "all mobile tests passed".into()),
            Some(code) => (Verdict::Fail, format!("appium tests exited with code {code}")),
            None => (Verdict::Error, "appium process terminated by signal".into()),
        };

        TestResult {
            capability: "appium".into(),
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
