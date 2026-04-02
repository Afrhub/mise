use mise_swarm::agent::{Capability, SpawnConfig};
use mise_swarm::execution::target::{TargetKind, TestTarget};
use mise_swarm::swarm::{self, Strategy};
use mise_swarm::topology::Topology;
use mise_swarm::SwarmConfig;

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let topology: Topology = args
        .iter()
        .position(|a| a == "--topology")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(Topology::Hierarchical);

    let strategy: Strategy = args
        .iter()
        .position(|a| a == "--strategy")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(Strategy::Auto);

    let max_agents: usize = args
        .iter()
        .position(|a| a == "--max-agents")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<usize>())
        .transpose()
        .map_err(|e| format!("invalid max-agents: {e}"))?
        .unwrap_or(8);

    let config = SwarmConfig {
        topology,
        strategy,
        max_agents,
    };

    let mut swarm = swarm::swarm_init(config)?;

    // Parse --spawn flags: --spawn type:name:cap1,cap2,cap3
    let spawn_indices: Vec<usize> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| *a == "--spawn")
        .map(|(i, _)| i)
        .collect();

    for idx in spawn_indices {
        if let Some(spec) = args.get(idx + 1) {
            let parts: Vec<&str> = spec.splitn(3, ':').collect();
            if parts.len() < 2 {
                return Err("--spawn format is type:name[:cap1,cap2,...]".into());
            }
            let agent_type = parts[0].parse()?;
            let name = parts[1].to_string();
            let capabilities: Vec<Capability> = if parts.len() == 3 {
                parts[2].split(',').map(|c| Capability::new(c.trim())).collect()
            } else {
                Vec::new()
            };

            swarm.spawn_agent(SpawnConfig {
                agent_type,
                name,
                capabilities,
            })?;
        }
    }

    // Parse --target flag: --target kind:name:locator
    let target = args
        .iter()
        .position(|a| a == "--target")
        .and_then(|i| args.get(i + 1))
        .map(|spec| parse_target(spec))
        .transpose()?;

    if let Some(target) = target {
        let results = swarm.execute(&target);
        println!("{}", serde_json::to_string_pretty(&results)?);
        if !results.succeeded() {
            std::process::exit(1);
        }
    } else {
        println!("{}", serde_json::to_string_pretty(&swarm)?);
    }

    Ok(())
}

fn parse_target(spec: &str) -> Result<TestTarget, Box<dyn std::error::Error>> {
    let parts: Vec<&str> = spec.splitn(3, ':').collect();
    if parts.len() < 3 {
        return Err("--target format is kind:name:locator (e.g. web-url:login:https://example.com)".into());
    }
    let kind: TargetKind = parts[0].parse().map_err(|e: String| e)?;
    Ok(TestTarget {
        name: parts[1].to_string(),
        kind,
        locator: parts[2].to_string(),
        env: std::collections::HashMap::new(),
    })
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
