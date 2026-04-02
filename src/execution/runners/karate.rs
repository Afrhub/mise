use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// API testing via Karate DSL (BDD-style, Cucumber-based).
pub struct KarateRunner;

impl ToolRunner for KarateRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "karate".into(),
            supported_kinds: vec![TargetKind::ApiEndpoint, TargetKind::Service],
            description: "API testing via Karate DSL (BDD-style HTTP testing)".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        // Karate can be invoked via java -jar or a standalone binary
        check_binary_on_path("karate", "karate")
            .or_else(|_| check_binary_on_path("java", "karate (via java)"))
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        let mut cmd = Command::new("karate");
        cmd.args(["-f", "json"]);
        cmd.env("KARATE_BASE_URL", &target.locator);
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
            Some(0) => (Verdict::Pass, parse_karate_summary(&stdout)),
            Some(code) => (Verdict::Fail, format!("karate exited with code {code}")),
            None => (Verdict::Error, "karate process terminated by signal".into()),
        };

        TestResult {
            capability: "karate".into(),
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

fn parse_karate_summary(stdout: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(stdout) {
        let passed = json["scenariosPassed"].as_u64().unwrap_or(0);
        let failed = json["scenariosFailed"].as_u64().unwrap_or(0);
        let total = json["featuresTotal"].as_u64().unwrap_or(0);
        format!("{passed} scenarios passed, {failed} failed across {total} features")
    } else {
        "all API tests passed".into()
    }
}
