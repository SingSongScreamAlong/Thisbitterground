# tbg_gdext - This Bitter Ground GDExtension

Godot 4 GDExtension bindings for the Rust ECS simulation.

## Building

```bash
cd gdext
cargo build --release
```

The compiled library will be at:
- **macOS**: `target/release/libtbg_gdext.dylib`
- **Linux**: `target/release/libtbg_gdext.so`
- **Windows**: `target/release/tbg_gdext.dll`

## Godot Setup

### 1. Create `.gdextension` File

Create `thisbitterground.gdextension` in your Godot project root:

```ini
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

[libraries]
macos.debug = "res://bin/libtbg_gdext.dylib"
macos.release = "res://bin/libtbg_gdext.dylib"
linux.debug.x86_64 = "res://bin/libtbg_gdext.so"
linux.release.x86_64 = "res://bin/libtbg_gdext.so"
windows.debug.x86_64 = "res://bin/tbg_gdext.dll"
windows.release.x86_64 = "res://bin/tbg_gdext.dll"
```

### 2. Copy the Library

Copy the compiled library to your Godot project's `bin/` folder.

### 3. Reload Godot

Restart Godot or reload the project. The `RustSimulation` and `SimWorldBridge` classes will be available.

## Usage

### RustSimulation (Recommended)

Lightweight class with efficient flat buffer snapshot API:

```gdscript
extends Node2D

const SQUAD_STRIDE = 14
const FACTION_BLUE = 0
const FACTION_RED = 1

var sim: RustSimulation
var unit_sprites: Dictionary = {}

func _ready():
    sim = RustSimulation.new()
    
    # Spawn some squads
    sim.spawn_mass_squads(FACTION_BLUE, -100, 0, 10, 200, 1)
    sim.spawn_mass_squads(FACTION_RED, 100, 0, 10, 200, 100)

func _process(delta):
    # Step the simulation (fixed timestep handled internally)
    sim.step(delta)
    
    # Get snapshot as flat buffer
    var buffer = sim.get_snapshot_buffer()
    var squad_count = int(buffer[0])
    
    # Update visuals
    for i in range(squad_count):
        var offset = 1 + i * SQUAD_STRIDE
        var squad_id = int(buffer[offset + 0])
        var x = buffer[offset + 1]
        var y = buffer[offset + 2]
        var faction = int(buffer[offset + 5])
        var health = buffer[offset + 7]
        var is_alive = buffer[offset + 11] > 0.5
        
        if is_alive:
            update_unit_visual(squad_id, x, y, faction, health)
        else:
            remove_unit_visual(squad_id)

func update_unit_visual(id, x, y, faction, health):
    # Your visualization code here
    pass

func remove_unit_visual(id):
    # Your cleanup code here
    pass
```

### Issuing Orders

```gdscript
# Move squad 1 to position (50, 100)
sim.issue_move_order(1, 50.0, 100.0)

# Attack-move squad 2 to position (0, 0)
sim.issue_attack_move_order(2, 0.0, 0.0)

# Hold position
sim.issue_hold_order(3)

# Retreat
sim.issue_retreat_order(4)
```

## Snapshot Buffer Format

The `get_snapshot_buffer()` method returns a `PackedFloat32Array`:

| Index | Field | Description |
|-------|-------|-------------|
| 0 | squad_count | Number of squads |

For each squad `i` at offset `1 + i * 14`:

| Offset | Field | Description |
|--------|-------|-------------|
| +0 | id | Squad ID |
| +1 | x | X position |
| +2 | y | Y position |
| +3 | vx | X velocity |
| +4 | vy | Y velocity |
| +5 | faction_id | 0=Blue, 1=Red |
| +6 | size | Squad size |
| +7 | health | Current health |
| +8 | health_max | Max health |
| +9 | morale | Morale (0.0-1.0) |
| +10 | suppression | Suppression (0.0-1.0) |
| +11 | is_alive | 1.0=alive, 0.0=dead |
| +12 | is_routing | 1.0=routing |
| +13 | order_type | 0=Hold, 1=Move, 2=Attack, 3=Retreat |

## Classes

### RustSimulation

Lightweight class optimized for real-time visualization:

| Method | Description |
|--------|-------------|
| `step(delta: float)` | Advance simulation (fixed timestep internal) |
| `get_snapshot_buffer() -> PackedFloat32Array` | Get state as flat buffer |
| `get_tick() -> int` | Current simulation tick |
| `get_time() -> float` | Elapsed simulation time |
| `issue_move_order(id, x, y)` | Move squad to position |
| `issue_attack_move_order(id, x, y)` | Attack-move to position |
| `issue_hold_order(id)` | Hold position |
| `issue_retreat_order(id)` | Retreat |
| `spawn_squad(id, faction, x, y)` | Spawn single squad |
| `spawn_mass_squads(faction, cx, cy, count, spread, start_id)` | Spawn formation |
| `get_squad_count() -> int` | Number of squads |
| `get_squad_stride() -> int` | Buffer stride (14) |
| `get_header_size() -> int` | Buffer header size (1) |

### SimWorldBridge

Full-featured bridge with JSON API (for debugging/prototyping):

| Method | Description |
|--------|-------------|
| `init_world()` | Initialize with test squads |
| `init_empty_world()` | Initialize empty |
| `step(delta)` | Advance simulation |
| `get_snapshot_json() -> String` | Get state as JSON |
| `get_snapshot_buffer() -> PackedFloat32Array` | Get state as flat buffer |
| `order_move(id, x, y)` | Move order |
| `order_attack_move(id, x, y)` | Attack-move order |
| `order_hold(id)` | Hold order |
| `order_retreat(id)` | Retreat order |
| `spawn_crater(x, y, radius, depth)` | Create terrain damage |
| `get_terrain_json() -> String` | Get terrain as JSON |

## Performance

- **30 Hz simulation** (default): Handles ~3000 units comfortably
- **20 Hz mode**: Available for larger battles (~5000 units)
- **Flat buffer**: More efficient than JSON for real-time updates
- **Fixed timestep**: Deterministic regardless of frame rate
