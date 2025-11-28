# This Bitter Ground — Rust ECS Edition

A large-scale stylized war simulation with a **Rust ECS backend** and **Godot 4 frontend**.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     GODOT 4 CLIENT                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Main.gd   │  │ Battlefield │  │  CameraController   │  │
│  │  (root)     │  │   .gd       │  │       .gd           │  │
│  └──────┬──────┘  └──────┬──────┘  └─────────────────────┘  │
│         │                │                                   │
│         └────────┬───────┘                                   │
│                  ▼                                           │
│         ┌───────────────┐                                    │
│         │  SimBridge.gd │  ◄── GDExtension (Phase 2+)        │
│         └───────┬───────┘                                    │
└─────────────────┼───────────────────────────────────────────┘
                  │ JSON Snapshots / Commands
                  ▼
┌─────────────────────────────────────────────────────────────┐
│                    RUST ECS SIMULATION                       │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   api.rs    │  │  world.rs   │  │    components.rs    │  │
│  │ (SimWorld)  │  │ (Snapshot)  │  │ (Position, Health…) │  │
│  └──────┬──────┘  └─────────────┘  └─────────────────────┘  │
│         │                                                    │
│         ▼                                                    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                    systems/                          │    │
│  │  movement.rs │ combat.rs │ morale.rs │ suppression  │    │
│  └─────────────────────────────────────────────────────┘    │
│                                                              │
│                      bevy_ecs                                │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
ThisBitterGround-RustECS/
├── sim/                    # Rust ECS simulation library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs          # Library entry point
│   │   ├── api.rs          # Public API (SimWorld)
│   │   ├── components.rs   # ECS components
│   │   ├── world.rs        # Snapshot types
│   │   └── systems/        # ECS systems
│   │       ├── movement.rs
│   │       ├── combat.rs
│   │       ├── morale.rs
│   │       ├── cover.rs
│   │       ├── suppression.rs
│   │       ├── terrain_damage.rs
│   │       └── serialization.rs
│   └── examples/
│       └── basic_demo.rs   # CLI demo
│
├── gdext/                  # GDExtension bindings (Phase 2)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Extension entry point
│       └── sim_bridge.rs   # SimWorldBridge class
│
├── client/                 # Godot 4 project
│   ├── project.godot
│   ├── bin/                # GDExtension libraries
│   │   ├── tbg_sim.gdextension
│   │   └── libtbg_gdext.dylib (macOS)
│   ├── scenes/
│   │   ├── Main.tscn
│   │   └── Battlefield.tscn
│   ├── scripts/
│   │   ├── Main.gd
│   │   ├── SimBridge.gd    # Auto-detects Rust/mock backend
│   │   ├── Battlefield.gd
│   │   ├── CameraController.gd
│   │   └── UnitVisualizer.gd
│   └── assets/
│
├── shared/                 # Shared protocol documentation
│   └── PROTOCOL.md
│
├── build.sh                # Build script
└── README.md
```

## Relationship to GDScript Prototype

This project is a **parallel implementation** of the original GDScript-only prototype located at:

```
/ThisBitterGround/          # Original GDScript prototype (preserved)
/ThisBitterGround-RustECS/  # This project (Rust ECS + Godot)
```

The original prototype serves as a reference implementation for game logic. Key systems being ported:
- Combat resolution
- Morale and suppression
- Squad movement and orders
- Sector control
- Fog of war / intel system

## Building

### Quick Build (All Components)

```bash
./build.sh
```

### Manual Build

#### Rust Simulation

```bash
cd sim
cargo build --release
cargo test
cargo run --example basic_demo
```

#### GDExtension

```bash
cd gdext
cargo build --release
cp target/release/libtbg_gdext.dylib ../client/bin/  # macOS
```

### Godot Client

1. Open Godot 4.3+
2. Import the project from `client/project.godot`
3. Run the Main scene

The client will automatically detect and use the Rust backend if available, otherwise it falls back to a GDScript mock implementation.

## Development Phases

| Phase | Description | Status |
|-------|-------------|--------|
| 0 | Project scaffold, basic ECS world | ✅ Complete |
| 1 | Core ECS systems (movement, combat, morale, cover) | ✅ Complete |
| 2 | GDExtension bridge (Rust ↔ Godot) | ✅ Complete |
| 3 | Camera & war-table view | ✅ Complete |
| 4 | Terrain & crater system | ✅ Complete |
| 5 | Destructibles (trees, buildings) | ✅ Complete |
| 6 | Swarm AI behavior | ✅ Complete |
| 7A | Spatial grid & mass unit scaling | ✅ Complete |
| 7B | Performance optimization pass | ✅ Complete |
| 8+ | Advanced features | Pending |

## Performance

The simulation has been optimized to handle 2000+ units at 49+ FPS:

- **Fixed Timestep**: 30 Hz deterministic simulation
- **LOD System**: Distant units update less frequently
- **Spatial Partitioning**: O(k) neighbor queries instead of O(n²)
- **Activity Flags**: Idle units skip expensive computations
- **Parallel Systems**: Independent systems run concurrently

See [PERFORMANCE.md](PERFORMANCE.md) for detailed benchmarks.

## Key Features (Planned)

- **Mass AI**: Hundreds to thousands of units via swarm/colony behavior ✅
- **Persistent Destruction**: Craters, deforestation, wreckage ✅
- **Living Terrain**: Heightmap deformation, mud buildup ✅
- **War Table View**: Zoom from overhead abstract view to 3D battlefield
- **Deterministic Simulation**: Fixed-timestep ECS for replay/multiplayer ✅

## Tech Stack

- **Backend**: Rust with `bevy_ecs` (ECS only, not full Bevy engine)
- **Frontend**: Godot 4.3+ with GDScript
- **Bridge**: GDExtension via `godot-rust` (Phase 2+)
- **Serialization**: `serde` + `serde_json`

## License

MIT
