# mise-swarm

Swarm topology initialization and agent coordination library.

## Topologies

- **hierarchical** — Binary tree. Node 0 is root coordinator.
- **mesh** — Fully connected. All nodes communicate directly.
- **ring** — Circular chain. Each node connects to its neighbors.
- **star** — Hub-and-spoke. Node 0 is the central hub.

## Agent Types

| Type | Description |
|------|-------------|
| `architect` | System design, database schema, API design |
| `coder` | Feature implementation |
| `tester` | Regression testing, security audit |
| `reviewer` | Code quality, best practices |
| `documenter` | API docs, user guides |

## CLI Usage

```bash
# Initialize a swarm with default settings (hierarchical, 8 nodes)
mise-swarm

# Specify topology and agent count
mise-swarm --topology star --strategy auto --max-agents 8

# Spawn typed agents with capabilities
mise-swarm --topology star --max-agents 8 \
  --spawn "architect:system-designer:database-schema,api-design" \
  --spawn "coder:feature-builder:react-native,typescript" \
  --spawn "tester:qa-agent:regression-testing,security-audit"
```

### Options

| Flag | Values | Default |
|------|--------|---------|
| `--topology` | `hierarchical`, `mesh`, `ring`, `star` | `hierarchical` |
| `--strategy` | `auto`, `round-robin`, `fill` | `auto` |
| `--max-agents` | any positive integer | `8` |
| `--spawn` | `type:name[:cap1,cap2,...]` | — |

## Library Usage

```rust
use mise_swarm::{SwarmConfig, swarm::swarm_init, agent::{SpawnConfig, Capability}};

let mut swarm = swarm_init(SwarmConfig::default()).unwrap();

swarm.spawn_agent(SpawnConfig {
    agent_type: "architect".parse().unwrap(),
    name: "designer".to_string(),
    capabilities: vec![Capability::new("api-design")],
}).unwrap();
```
