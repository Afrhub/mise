use std::process::Command;

use crate::execution::result::{TestResult, Verdict};
use crate::execution::runner::{check_binary_on_path, RunnerError, RunnerScope, ToolRunner};
use crate::execution::target::{TargetKind, TestTarget};

/// Security scanning via OWASP ZAP or Nikto.
pub struct SecurityAuditRunner;

impl ToolRunner for SecurityAuditRunner {
    fn scope(&self) -> RunnerScope {
        RunnerScope {
            capability: "security-audit".into(),
            supported_kinds: vec![TargetKind::WebUrl, TargetKind::ApiEndpoint],
            description: "Security vulnerability scanning via OWASP ZAP or Nikto".into(),
        }
    }

    fn check_available(&self) -> Result<(), RunnerError> {
        check_binary_on_path("zap-cli", "security-audit (OWASP ZAP)")
            .or_else(|_| check_binary_on_path("nikto", "security-audit (Nikto)"))
    }

    fn build_command(&self, target: &TestTarget) -> Result<Command, RunnerError> {
        // Prefer ZAP, fall back to Nikto
        if check_binary_on_path("zap-cli", "zap").is_ok() {
            let mut cmd = Command::new("zap-cli");
            cmd.args(["quick-scan", "--self-contained", "-o", "json", &target.locator]);
            for (k, v) in &target.env {
                cmd.env(k, v);
            }
            Ok(cmd)
        } else {
            let mut cmd = Command::new("nikto");
            cmd.args(["-h", &target.locator, "-Format", "json", "-output", "-"]);
            for (k, v) in &target.env {
                cmd.env(k, v);
            }
            Ok(cmd)
        }
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
            // ZAP JSON format
            let high = json["alerts"]
                .as_array()
                .map(|a| a.iter().filter(|v| v["risk"] == "High").count())
                .unwrap_or(0);
            let medium = json["alerts"]
                .as_array()
                .map(|a| a.iter().filter(|v| v["risk"] == "Medium").count())
                .unwrap_or(0);
            if high > 0 {
                (Verdict::Fail, format!("{high} high, {medium} medium vulnerabilities found"))
            } else if medium > 0 {
                (Verdict::Pass, format!("0 high, {medium} medium vulnerabilities (advisory)"))
            } else {
                (Verdict::Pass, "no vulnerabilities found".into())
            }
        } else {
            match exit_code {
                Some(0) => (Verdict::Pass, "security scan passed".into()),
                Some(code) => (Verdict::Fail, format!("security scan exited with code {code}")),
                None => (Verdict::Error, "security scan terminated by signal".into()),
            }
        };

        TestResult {
            capability: "security-audit".into(),
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
