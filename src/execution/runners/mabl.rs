use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Cloud-based intelligent testing via Mabl.
///
/// Requires the `mabl` CLI and a valid API key set in the environment
/// (MABL_API_KEY) or passed through the target's env map.
pub struct MablRunner;

impl ToolRunner for MablRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "mabl".into(),
            supported_kinds: vec![TargetKind::WebUrl, TargetKind::ApiEndpoint],
            description: "Cloud-based intelligent testing via Mabl".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("mabl", "mabl")
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        let mut cmd = Command::new("mabl");
        cmd.args([
            "deployments",
            "create",
            "--await-completion",
            "--output=json",
            "--url",
            &target.locator,
        ]);
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

        let (verdict, summary) = if let Ok(json) =
            serde_json::from_str::<serde_json::Value>(&stdout)
        {
            let status = json["plan_execution"]["status"]
                .as_str()
                .unwrap_or("unknown");
            match status {
                "succeeded" => (Verdict::Pass, format!("mabl deployment test succeeded")),
                "failed" => (Verdict::Fail, format!("mabl deployment test failed")),
                other => (Verdict::Error, format!("mabl status: {other}")),
            }
        } else {
            match exit_code {
                Some(0) => (Verdict::Pass, "mabl tests passed".into()),
                Some(code) => (Verdict::Fail, format!("mabl exited with code {code}")),
                None => (Verdict::Error, "mabl process terminated by signal".into()),
            }
        };

        TestResult {
            capability: "mabl".into(),
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
