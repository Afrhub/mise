use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Browser testing via Docker-Selenium grid.
///
/// Spins up a Selenium container, runs tests against it, then tears down.
pub struct DockerSeleniumRunner;

impl ToolRunner for DockerSeleniumRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "docker-selenium".into(),
            supported_kinds: vec![TargetKind::WebUrl],
            description: "Browser testing via Selenium Grid in Docker containers".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("docker", "docker-selenium")
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        // Run selenium tests inside a docker container with the selenium/standalone-chrome image
        let mut cmd = Command::new("docker");
        cmd.args([
            "run",
            "--rm",
            "--network=host",
            "-e",
            &format!("BASE_URL={}", target.locator),
        ]);
        // Pass through any extra env vars
        for (k, v) in &target.env {
            cmd.args(["-e", &format!("{k}={v}")]);
        }
        cmd.args([
            "selenium/standalone-chrome:latest",
            "bash",
            "-c",
            "selenium-side-runner --server http://localhost:4444 /tests/*.side",
        ]);
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
            Some(0) => (Verdict::Pass, "all selenium tests passed".into()),
            Some(code) => (
                Verdict::Fail,
                format!("docker-selenium exited with code {code}"),
            ),
            None => (
                Verdict::Error,
                "docker-selenium process terminated by signal".into(),
            ),
        };

        TestResult {
            capability: "docker-selenium".into(),
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
