use serde::{Deserialize, Serialize};

use super::target::TestTarget;

/// Pass/fail/skip/error verdict for a single runner execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    Pass,
    Fail,
    Skip,
    Error,
}

/// Result from a single tool runner execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Which capability/runner produced this result.
    pub capability: String,
    /// Which agent ran it.
    pub agent_name: String,
    pub verdict: Verdict,
    /// Summary message (e.g. "12 tests passed, 1 failed").
    pub summary: String,
    /// Captured stdout from the subprocess.
    pub stdout: String,
    /// Captured stderr from the subprocess.
    pub stderr: String,
    /// Exit code from the subprocess.
    pub exit_code: Option<i32>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Optional coverage percentage (0.0-100.0).
    pub coverage_pct: Option<f64>,
}

/// Aggregated results across all runners for a target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedResults {
    pub target: TestTarget,
    pub results: Vec<TestResult>,
    pub total_pass: usize,
    pub total_fail: usize,
    pub total_skip: usize,
    pub total_error: usize,
    pub overall_coverage_pct: Option<f64>,
}

impl AggregatedResults {
    pub fn aggregate(target: TestTarget, results: Vec<TestResult>) -> Self {
        let total_pass = results.iter().filter(|r| r.verdict == Verdict::Pass).count();
        let total_fail = results.iter().filter(|r| r.verdict == Verdict::Fail).count();
        let total_skip = results.iter().filter(|r| r.verdict == Verdict::Skip).count();
        let total_error = results.iter().filter(|r| r.verdict == Verdict::Error).count();

        let coverages: Vec<f64> = results
            .iter()
            .filter_map(|r| r.coverage_pct)
            .collect();
        let overall_coverage_pct = if coverages.is_empty() {
            None
        } else {
            Some(coverages.iter().sum::<f64>() / coverages.len() as f64)
        };

        Self {
            target,
            results,
            total_pass,
            total_fail,
            total_skip,
            total_error,
            overall_coverage_pct,
        }
    }

    pub fn succeeded(&self) -> bool {
        self.total_fail == 0 && self.total_error == 0
    }
}
