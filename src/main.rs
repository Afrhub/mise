use mise_swarm::agent::{AgentType, Capability, SpawnConfig};
use mise_swarm::swarm::{self, Strategy};
use mise_swarm::topology::Topology;
use mise_swarm::SwarmConfig;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let topology = args
        .iter()
        .position(|a| a == "--topology")
        .and_then(|i| args.get(i + 1))
        .map(|s| serde_json::from_value(serde_json::Value::String(s.clone())).unwrap())
        .unwrap_or(Topology::Hierarchical);

    let strategy = args
        .iter()
        .position(|a| a == "--strategy")
        .and_then(|i| args.get(i + 1))
        .map(|s| Strategy::from_str(s).expect("invalid strategy"))
        .unwrap_or(Strategy::Auto);

    let max_agents = args
        .iter()
        .position(|a| a == "--max-agents")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.parse::<usize>().expect("invalid max-agents"))
        .unwrap_or(8);

    let config = SwarmConfig {
        topology,
        strategy,
        max_agents,
    };

    let mut swarm = match swarm::swarm_init(config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    };

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
                eprintln!("error: --spawn format is type:name[:cap1,cap2,...]");
                std::process::exit(1);
            }
            let agent_type = AgentType::from_str(parts[0]).unwrap_or_else(|e| {
                eprintln!("error: {e}");
                std::process::exit(1);
            });
            let name = parts[1].to_string();
            let capabilities: Vec<Capability> = if parts.len() == 3 {
                parts[2].split(',').map(|c| Capability::new(c.trim())).collect()
            } else {
                Vec::new()
            };

            if let Err(e) = swarm.spawn_agent(SpawnConfig {
                agent_type,
                name,
                capabilities,
            }) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&swarm).unwrap());
}
