use mise_swarm::{swarm, SwarmConfig};
use mise_swarm::topology::Topology;
use mise_swarm::swarm::Strategy;

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

    match swarm::swarm_init(config) {
        Ok(swarm) => {
            println!("{}", serde_json::to_string_pretty(&swarm).unwrap());
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
