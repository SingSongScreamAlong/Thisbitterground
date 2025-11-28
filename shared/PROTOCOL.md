# This Bitter Ground — Protocol Specification

## Overview

This document describes the data protocol between the Rust ECS simulation (`/sim`) and the Godot client (`/client`).

## Current Phase: 0-1 (Mock Implementation)

In Phases 0-1, the Godot client uses a mock `SimBridge` that simulates the Rust backend behavior in GDScript. This allows development of the visualization layer before the GDExtension bridge is complete.

## Snapshot Format

The simulation state is communicated via a `Snapshot` structure, serialized as JSON.

### Snapshot Schema

```json
{
  "tick": 123,
  "time": 6.15,
  "squads": [
    {
      "id": 0,
      "faction": "Blue",
      "x": -45.2,
      "y": 10.0,
      "vx": 5.0,
      "vy": 0.0,
      "health": 85.0,
      "health_max": 100.0,
      "size": 12,
      "morale": 0.8,
      "suppression": 0.3,
      "order": "MoveTo(0.0,10.0)"
    }
  ],
  "terrain_damage": [
    {
      "x": 10.0,
      "y": 20.0,
      "radius": 5.0,
      "depth": 1.0
    }
  ]
}
```

### Field Descriptions

#### Root Fields

| Field | Type | Description |
|-------|------|-------------|
| `tick` | u64 | Current simulation tick number |
| `time` | f32 | Elapsed simulation time in seconds |
| `squads` | Array | List of squad states |
| `terrain_damage` | Array | Terrain damage events this tick |

#### Squad Fields

| Field | Type | Description |
|-------|------|-------------|
| `id` | u32 | Unique squad identifier |
| `faction` | String | "Blue" or "Red" |
| `x`, `y` | f32 | Position on battlefield (2D) |
| `vx`, `vy` | f32 | Current velocity |
| `health` | f32 | Current health points |
| `health_max` | f32 | Maximum health points |
| `size` | u32 | Number of soldiers in squad |
| `morale` | f32 | Morale level (0.0 - 1.0) |
| `suppression` | f32 | Suppression level (0.0+) |
| `order` | String | Current order description |

#### Terrain Damage Fields

| Field | Type | Description |
|-------|------|-------------|
| `x`, `y` | f32 | Center position of damage |
| `radius` | f32 | Radius of affected area |
| `depth` | f32 | Depth of crater/damage |

## Commands (Godot → Rust)

### Order Commands

```gdscript
# Move to position
sim_bridge.order_move(squad_id: int, target_x: float, target_y: float)

# Attack-move to position
sim_bridge.order_attack_move(squad_id: int, target_x: float, target_y: float)

# Hold position
sim_bridge.order_hold(squad_id: int)
```

### Simulation Control

```gdscript
# Initialize world
sim_bridge.init_world()

# Step simulation
sim_bridge.step(delta: float)

# Get current snapshot
var snapshot: Dictionary = sim_bridge.get_snapshot()
```

## Coordinate System

- **Simulation (Rust)**: Uses 2D coordinates (x, y) where:
  - x = east/west axis
  - y = north/south axis

- **Visualization (Godot)**: Uses 3D coordinates (x, y, z) where:
  - x = east/west (same as sim x)
  - y = vertical height (always 0 for ground units)
  - z = north/south (same as sim y)

## Future Phases

### Phase 2: GDExtension Bridge

The mock `SimBridge` will be replaced with actual Rust bindings via `godot-rust`:

```rust
#[gdextension]
impl SimWorldBridge {
    fn init_world(&mut self);
    fn step(&mut self, dt: f64);
    fn get_snapshot_json(&self) -> GString;
    fn order_move(&mut self, squad_id: i32, x: f32, y: f32);
}
```

### Phase 3+: Binary Protocol

For performance, JSON may be replaced with a binary format (e.g., MessagePack or custom binary) for snapshot transmission.
