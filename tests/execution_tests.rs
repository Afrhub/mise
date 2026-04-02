use mise_swarm::agent::{AgentType, Capability, SpawnConfig};
use mise_swarm::execution::coverage::CoverageResolver;
use mise_swarm::execution::result::{AggregatedResults, TestResult, Verdict};
use mise_swarm::execution::runner::ToolRunner;
use mise_swarm::execution::runners;
use mise_swarm::execution::target::{TargetKind, TestTarget};
use mise_swarm::execution::CapabilityDispatcher;
use mise_swarm::swarm::{swarm_init, SwarmConfig};
use mise_swarm::topology::Topology;

use std::collections::HashMap;

fn test_target(kind: TargetKind) -> TestTarget {
    TestTarget {
        name: "test-target".into(),
        kind,
        locator: "https://example.com".into(),
        env: HashMap::new(),
    }
}

// --- TargetKind parsing ---

#[test]
fn target_kind_from_str() {
    assert_eq!("web-url".parse::<TargetKind>().unwrap(), TargetKind::WebUrl);
    assert_eq!("url".parse::<TargetKind>().unwrap(), TargetKind::WebUrl);
    assert_eq!("api".parse::<TargetKind>().unwrap(), TargetKind::ApiEndpoint);
    assert_eq!(
        "api-endpoint".parse::<TargetKind>().unwrap(),
        TargetKind::ApiEndpoint
    );
    assert_eq!(
        "mobile-app".parse::<TargetKind>().unwrap(),
        TargetKind::MobileApp
    );
    assert_eq!(
        "mobile".parse::<TargetKind>().unwrap(),
        TargetKind::MobileApp
    );
    assert_eq!("service".parse::<TargetKind>().unwrap(), TargetKind::Service);
    assert_eq!("svc".parse::<TargetKind>().unwrap(), TargetKind::Service);
    assert!("nonsense".parse::<TargetKind>().is_err());
}

// --- Runner scope correctness ---

#[test]
fn all_runners_registered() {
    let runners = runners::default_runners();
    let expected = [
        "playwright",
        "karate",
        "appium",
        "docker-selenium",
        "qa-wolf",
        "mabl",
        "regression-testing",
        "security-audit",
        "edge-cases",
    ];
    for name in expected {
        assert!(
            runners.contains_key(name),
            "runner '{name}' not registered"
        );
    }
    assert_eq!(runners.len(), 9);
}

#[test]
fn runner_scope_matches_key() {
    let runners = runners::default_runners();
    for (key, runner) in &runners {
        assert_eq!(
            &runner.scope().capability,
            key,
            "scope capability mismatch for '{key}'"
        );
    }
}

#[test]
fn playwright_supports_web_and_api() {
    let runners = runners::default_runners();
    let scope = runners["playwright"].scope();
    assert!(scope.supported_kinds.contains(&TargetKind::WebUrl));
    assert!(scope.supported_kinds.contains(&TargetKind::ApiEndpoint));
    assert!(!scope.supported_kinds.contains(&TargetKind::MobileApp));
}

#[test]
fn karate_supports_api_and_service() {
    let runners = runners::default_runners();
    let scope = runners["karate"].scope();
    assert!(scope.supported_kinds.contains(&TargetKind::ApiEndpoint));
    assert!(scope.supported_kinds.contains(&TargetKind::Service));
    assert!(!scope.supported_kinds.contains(&TargetKind::WebUrl));
}

#[test]
fn appium_supports_mobile_only() {
    let runners = runners::default_runners();
    let scope = runners["appium"].scope();
    assert_eq!(scope.supported_kinds, vec![TargetKind::MobileApp]);
}

#[test]
fn docker_selenium_supports_web_only() {
    let runners = runners::default_runners();
    let scope = runners["docker-selenium"].scope();
    assert_eq!(scope.supported_kinds, vec![TargetKind::WebUrl]);
}

#[test]
fn regression_supports_all_kinds() {
    let runners = runners::default_runners();
    let scope = runners["regression-testing"].scope();
    assert_eq!(scope.supported_kinds.len(), 4);
}

#[test]
fn edge_cases_supports_all_kinds() {
    let runners = runners::default_runners();
    let scope = runners["edge-cases"].scope();
    assert_eq!(scope.supported_kinds.len(), 4);
}

// --- Coverage resolver ---

#[test]
fn coverage_resolver_filters_by_target_kind() {
    let runners = runners::default_runners();
    let resolver = CoverageResolver::new(&runners);

    // Create a mock agent with playwright + appium
    let agent = mise_swarm::agent::SpawnedAgent {
        id: 0,
        agent_type: AgentType::Tester,
        name: "test-agent".into(),
        capabilities: vec![Capability::new("playwright"), Capability::new("appium")],
        status: mise_swarm::agent::AgentStatus::Idle,
        node: 0,
    };

    // WebUrl target: only playwright should match (appium doesn't support web)
    let web_target = test_target(TargetKind::WebUrl);
    let web_runners = resolver.resolve(&agent, &web_target);
    assert_eq!(web_runners.len(), 1);
    assert_eq!(web_runners[0].scope().capability, "playwright");

    // MobileApp target: only appium should match
    let mobile_target = test_target(TargetKind::MobileApp);
    let mobile_runners = resolver.resolve(&agent, &mobile_target);
    assert_eq!(mobile_runners.len(), 1);
    assert_eq!(mobile_runners[0].scope().capability, "appium");
}

#[test]
fn coverage_resolver_reports_unresolved_capabilities() {
    let runners = runners::default_runners();
    let resolver = CoverageResolver::new(&runners);

    let agent = mise_swarm::agent::SpawnedAgent {
        id: 0,
        agent_type: AgentType::Tester,
        name: "test-agent".into(),
        capabilities: vec![
            Capability::new("playwright"),
            Capability::new("unknown-tool"),
            Capability::new("another-unknown"),
        ],
        status: mise_swarm::agent::AgentStatus::Idle,
        node: 0,
    };

    let unresolved = resolver.unresolved_capabilities(&agent);
    assert_eq!(unresolved.len(), 2);
    assert!(unresolved.contains(&"unknown-tool".to_string()));
    assert!(unresolved.contains(&"another-unknown".to_string()));
}

// --- Aggregated results ---

#[test]
fn aggregation_counts_verdicts() {
    let target = test_target(TargetKind::WebUrl);
    let results = vec![
        TestResult {
            capability: "a".into(),
            agent_name: "ag".into(),
            verdict: Verdict::Pass,
            summary: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(0),
            duration_ms: 100,
            coverage_pct: Some(80.0),
        },
        TestResult {
            capability: "b".into(),
            agent_name: "ag".into(),
            verdict: Verdict::Fail,
            summary: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: Some(1),
            duration_ms: 200,
            coverage_pct: Some(60.0),
        },
        TestResult {
            capability: "c".into(),
            agent_name: "ag".into(),
            verdict: Verdict::Skip,
            summary: String::new(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
            duration_ms: 0,
            coverage_pct: None,
        },
    ];

    let agg = AggregatedResults::aggregate(target, results);
    assert_eq!(agg.total_pass, 1);
    assert_eq!(agg.total_fail, 1);
    assert_eq!(agg.total_skip, 1);
    assert_eq!(agg.total_error, 0);
    assert!(!agg.succeeded());

    // Average of 80.0 and 60.0 = 70.0
    assert!((agg.overall_coverage_pct.unwrap() - 70.0).abs() < 0.01);
}

#[test]
fn aggregation_succeeds_when_all_pass() {
    let target = test_target(TargetKind::WebUrl);
    let results = vec![TestResult {
        capability: "a".into(),
        agent_name: "ag".into(),
        verdict: Verdict::Pass,
        summary: String::new(),
        stdout: String::new(),
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 50,
        coverage_pct: None,
    }];

    let agg = AggregatedResults::aggregate(target, results);
    assert!(agg.succeeded());
    assert!(agg.overall_coverage_pct.is_none());
}

// --- Dispatcher integration ---

#[test]
fn dispatcher_produces_results_for_all_capabilities() {
    let dispatcher = CapabilityDispatcher::new();

    let agent = mise_swarm::agent::SpawnedAgent {
        id: 0,
        agent_type: AgentType::Tester,
        name: "qa-agent".into(),
        capabilities: vec![
            Capability::new("playwright"),
            Capability::new("karate"),
            Capability::new("appium"),
            Capability::new("docker-selenium"),
            Capability::new("qa-wolf"),
            Capability::new("mabl"),
            Capability::new("regression-testing"),
            Capability::new("security-audit"),
            Capability::new("edge-cases"),
        ],
        status: mise_swarm::agent::AgentStatus::Idle,
        node: 0,
    };

    // WebUrl target: should match playwright, docker-selenium, qa-wolf, mabl,
    // regression-testing, security-audit, edge-cases (7 runners)
    // appium and karate should NOT match (wrong target kind) and won't appear
    let target = test_target(TargetKind::WebUrl);
    let results = dispatcher.execute_for_agent(&agent, &target);

    // Each result should have a capability name set
    let caps: Vec<&str> = results.iter().map(|r| r.capability.as_str()).collect();

    // These should be present (either run or error because tool not installed)
    assert!(caps.contains(&"playwright"), "missing playwright: {caps:?}");
    assert!(
        caps.contains(&"docker-selenium"),
        "missing docker-selenium: {caps:?}"
    );
    assert!(caps.contains(&"qa-wolf"), "missing qa-wolf: {caps:?}");
    assert!(caps.contains(&"mabl"), "missing mabl: {caps:?}");
    assert!(
        caps.contains(&"regression-testing"),
        "missing regression-testing: {caps:?}"
    );
    assert!(
        caps.contains(&"security-audit"),
        "missing security-audit: {caps:?}"
    );
    assert!(caps.contains(&"edge-cases"), "missing edge-cases: {caps:?}");

    // Appium and karate don't support WebUrl, so they should not appear
    assert!(!caps.contains(&"appium"), "appium should not run for WebUrl");
    assert!(!caps.contains(&"karate"), "karate should not run for WebUrl");
}

#[test]
fn dispatcher_skips_unregistered_capabilities() {
    let dispatcher = CapabilityDispatcher::new();

    let agent = mise_swarm::agent::SpawnedAgent {
        id: 0,
        agent_type: AgentType::Tester,
        name: "test".into(),
        capabilities: vec![Capability::new("nonexistent-tool")],
        status: mise_swarm::agent::AgentStatus::Idle,
        node: 0,
    };

    let target = test_target(TargetKind::WebUrl);
    let results = dispatcher.execute_for_agent(&agent, &target);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].capability, "nonexistent-tool");
    assert_eq!(results[0].verdict, Verdict::Skip);
}

// --- Swarm-level execution ---

#[test]
fn swarm_execute_runs_across_all_agents() {
    let mut swarm = swarm_init(SwarmConfig {
        topology: Topology::Star,
        strategy: mise_swarm::swarm::Strategy::Auto,
        max_agents: 4,
    })
    .unwrap();

    swarm
        .spawn_agent(SpawnConfig {
            agent_type: AgentType::Tester,
            name: "tester-1".into(),
            capabilities: vec![Capability::new("playwright")],
        })
        .unwrap();

    swarm
        .spawn_agent(SpawnConfig {
            agent_type: AgentType::Tester,
            name: "tester-2".into(),
            capabilities: vec![Capability::new("karate")],
        })
        .unwrap();

    let target = test_target(TargetKind::ApiEndpoint);
    let results = swarm.execute(&target);

    // Both agents should produce results
    let agent_names: Vec<&str> = results
        .results
        .iter()
        .map(|r| r.agent_name.as_str())
        .collect();
    assert!(agent_names.contains(&"tester-1"));
    assert!(agent_names.contains(&"tester-2"));
}

// --- has_capability ---

#[test]
fn spawned_agent_has_capability() {
    let agent = mise_swarm::agent::SpawnedAgent {
        id: 0,
        agent_type: AgentType::Tester,
        name: "t".into(),
        capabilities: vec![Capability::new("playwright"), Capability::new("karate")],
        status: mise_swarm::agent::AgentStatus::Idle,
        node: 0,
    };

    assert!(agent.has_capability("playwright"));
    assert!(agent.has_capability("karate"));
    assert!(!agent.has_capability("appium"));
}
