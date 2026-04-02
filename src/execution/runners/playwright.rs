use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Browser-based E2E and API testing via Playwright.
pub struct PlaywrightRunner;

impl ToolRunner for PlaywrightRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "playwright".into(),
            supported_kinds: vec![TargetKind::WebUrl, TargetKind::ApiEndpoint],
            description: "Browser-based E2E and API testing via Playwright".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("npx", "playwright")
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        let mut cmd = Command::new("npx");
        cmd.args(["playwright", "test", "--reporter=json"]);
        cmd.env("BASE_URL", &target.locator);
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
            Some(0) => (Verdict::Pass, parse_playwright_summary(&stdout)),
            Some(code) => (Verdict::Fail, format!("playwright exited with code {code}")),
            None => (Verdict::Error, "playwright process terminated by signal".into()),
        };

        TestResult {
            capability: "playwright".into(),
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

fn parse_playwright_summary(stdout: &str) -> String {
    // Attempt to parse Playwright JSON reporter output
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
        let suites = json["suites"].as_array().map(|s| s.len()).unwrap_or(0);
        let passed = count_specs_by_status(&json, "expected");
        let failed = count_specs_by_status(&json, "unexpected");
        format!("{passed} passed, {failed} failed across {suites} suites")
    } else {
        "all tests passed".into()
    }
}

fn count_specs_by_status(json: &serde_json::Value, status: &str) -> usize {
    json["suites"]
        .as_array()
        .map(|suites| {
            suites
                .iter()
                .flat_map(|s| s["specs"].as_array().into_iter().flatten())
                .filter(|spec| spec["ok"].as_bool() == Some(status == "expected"))
                .count()
        })
        .unwrap_or(0)
}
