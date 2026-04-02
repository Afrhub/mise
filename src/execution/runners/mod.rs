pub mod appium;
pub mod docker_selenium;
pub mod edge_cases;
pub mod karate;
pub mod mabl;
pub mod playwright;
pub mod qa_wolf;
pub mod regression;
pub mod security;

use std::collections::HashMap;

use super::runner::ToolRunner;

/// Build the default set of all known runners, keyed by capability string.
pub fn default_runners() -> HashMap<String, Box<dyn ToolRunner>> {
    let runners: Vec<Box<dyn ToolRunner>> = vec![
        Box::new(playwright::PlaywrightRunner),
        Box::new(karate::KarateRunner),
        Box::new(appium::AppiumRunner),
        Box::new(docker_selenium::DockerSeleniumRunner),
        Box::new(qa_wolf::QaWolfRunner),
        Box::new(mabl::MablRunner),
        Box::new(regression::RegressionRunner),
        Box::new(security::SecurityAuditRunner),
        Box::new(edge_cases::EdgeCasesRunner),
    ];
    runners
        .into_iter()
        .map(|r| (r.scope().capability.clone(), r))
        .collect()
}
