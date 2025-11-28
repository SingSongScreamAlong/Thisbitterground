# This Bitter Ground - Godot Project

Minimal Godot 4 visualization for the Rust ECS simulation.

## Prerequisites

- Godot 4.2+ (download from [godotengine.org](https://godotengine.org/download))
- Rust toolchain (for building the GDExtension)

## Building the GDExtension

From the repository root:

```bash
cargo build --release -p tbg_gdext
```

This produces the native library at:

| Platform | Output Path |
|----------|-------------|
| macOS | `gdext/target/release/libtbg_gdext.dylib` |
| Linux | `gdext/target/release/libtbg_gdext.so` |
| Windows | `gdext/target/release/tbg_gdext.dll` |

The `.gdextension` file is already configured to look for the library at these relative paths.

## Running the Project

1. **Build the Rust library** (see above)

2. **Open in Godot**:
   - Launch Godot 4.2+
   - Click "Import"
   - Navigate to `godot/` folder and select `project.godot`
   - Click "Import & Edit"

3. **Run the main scene**:
   - Press F5 or click the Play button
   - You should see:
     - Blue circles on the left (Blue faction squads)
     - Red circles on the right (Red faction squads)
     - Squads moving toward each other and fighting
     - Debug overlay showing FPS, tick count, and squad count

## What You're Seeing

The visualization shows:

- **Blue circles**: Blue faction squads
- **Red circles**: Red faction squads  
- **Yellow circles**: Routing (fleeing) squads
- **Gray circles**: Dead squads
- **Opacity**: Reflects current health (lower health = more transparent)

The Rust simulation runs at 30 Hz internally. Godot calls `step(delta)` each frame, and the Rust side handles fixed-timestep accumulation.

## Project Structure

```
godot/
├── project.godot              # Godot project config
├── thisbitterground.gdextension  # GDExtension config (loads Rust library)
├── icon.svg                   # Project icon
├── scenes/
│   └── Main.tscn              # Main scene
├── scripts/
│   └── Main.gd                # Main controller script
└── bin/                       # (Optional) Copy compiled library here for distribution
```

## GDExtension Classes

The Rust library exposes two classes:

### RustSimulation (Recommended)

Lightweight class optimized for real-time visualization:

```gdscript
var sim = RustSimulation.new()

# Step simulation
sim.step(delta)

# Get snapshot as flat buffer
var buffer = sim.get_snapshot_buffer()
var squad_count = int(buffer[0])

# Spawn squads
sim.spawn_squad(id, faction, x, y)
sim.spawn_mass_squads(faction, cx, cy, count, spread, start_id)

# Issue orders
sim.issue_move_order(id, x, y)
sim.issue_attack_move_order(id, x, y)
sim.issue_hold_order(id)
sim.issue_retreat_order(id)
```

### SimWorldBridge

Full-featured bridge with JSON API (for debugging):

```gdscript
var bridge = SimWorldBridge.new()
bridge.init_world()  # Initialize with test squads
bridge.step(delta)
var json = bridge.get_snapshot_json()
```

## Snapshot Buffer Format

The `get_snapshot_buffer()` method returns a `PackedFloat32Array`:

```
[0] = squad_count

For each squad i at offset (1 + i * 14):
  [+0]  id          - Squad ID
  [+1]  x           - X position
  [+2]  y           - Y position
  [+3]  vx          - X velocity
  [+4]  vy          - Y velocity
  [+5]  faction_id  - 0=Blue, 1=Red
  [+6]  size        - Squad size
  [+7]  health      - Current health
  [+8]  health_max  - Max health
  [+9]  morale      - Morale (0.0-1.0)
  [+10] suppression - Suppression (0.0-1.0)
  [+11] is_alive    - 1.0=alive, 0.0=dead
  [+12] is_routing  - 1.0=routing
  [+13] order_type  - 0=Hold, 1=Move, 2=Attack, 3=Retreat
```

## Troubleshooting

### "Failed to create RustSimulation"

The GDExtension library isn't loading. Check:

1. Did you build with `cargo build --release -p tbg_gdext`?
2. Is the library at the path specified in `thisbitterground.gdextension`?
3. Check Godot's console for error messages.

### Library not found

The `.gdextension` file uses relative paths to `../gdext/target/release/`. If you moved the project, update the paths or copy the library to `godot/bin/` and uncomment the alternative paths in the `.gdextension` file.

### Squads not moving

Make sure you're calling `sim.step(delta)` in `_process()`. The simulation only advances when stepped.
