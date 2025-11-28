# Test Results Report

**Generated**: November 28, 2025  
**System**: Apple Silicon Mac  
**Rust Version**: stable  
**Build Profile**: Release (`--release`)

---

## How to Reproduce

All results can be independently verified by running:

```bash
cd /path/to/ThisBitterGround-RustECS/sim

# Run all tests with output
cargo test --release -- --nocapture

# Run only stress tests with timing
cargo test test_stress --release -- --nocapture

# Run specific test
cargo test test_stress_1000_units --release -- --nocapture
cargo test test_stress_2000_units --release -- --nocapture
```

---

## Full Test Suite Results

**Command**: `cargo test --release`

```
running 34 tests
test spatial::tests::test_faction_queries ... ok
test spatial::tests::test_nearest_enemy ... ok
test spatial::tests::test_spatial_grid_insert_query ... ok
test systems::ai::tests::test_behavior_state_transitions ... ok
test systems::ai::tests::test_threat_awareness_detects_enemies ... ok
test systems::cover::tests::test_cover_damage_reduction ... ok
test systems::combat::tests::test_combat_applies_damage_and_suppression ... ok
test api::tests::test_new_world ... ok
test api::tests::test_snapshot_json ... ok
test api::tests::test_default_test_world ... ok
test systems::cover::tests::test_cover_zones ... ok
test systems::destruction::tests::test_destruction_state_transitions ... ok
test systems::morale::tests::test_morale_decreases_when_suppressed ... ok
test systems::destruction::tests::test_terrain_damage_affects_destructibles ... ok
test systems::morale::tests::test_morale_recovers_when_not_suppressed ... ok
test systems::morale::tests::test_rout_when_broken ... ok
test systems::movement::tests::test_movement_applies_velocity ... ok
test api::tests::test_step_advances_tick ... ok
test systems::performance::tests::test_lod_assignment ... ok
test systems::performance::tests::test_sim_tick_lod_scheduling ... ok
test systems::performance::tests::test_sector_assignment ... ok
test systems::serialization::tests::test_snapshot_roundtrip ... ok
test systems::suppression::tests::test_suppression_decays ... ok
test terrain::tests::test_movement_multiplier ... ok
test terrain::tests::test_cover_values ... ok
test terrain::tests::test_crater_application ... ok
test systems::terrain_damage::tests::test_terrain_damage_cleared ... ok
test terrain::tests::test_world_to_grid ... ok
test terrain::tests::test_terrain_grid_creation ... ok
test api::tests::test_spatial_grid_populated ... ok
test api::tests::test_move_order ... ok
test api::tests::test_mass_spawn_and_step ... ok
test api::tests::test_stress_2000_units ... ok
test api::tests::test_stress_1000_units ... ok

test result: ok. 34 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

**Total**: 34 tests  
**Passed**: 34  
**Failed**: 0  
**Ignored**: 0

---

## Stress Test Results

### Test: `test_stress_1000_units`

**Configuration**:
- Units: 1000 (500 Blue + 500 Red)
- Fixed timestep: 20 Hz (1/20 = 0.05s)
- Game time simulated: 5.0 seconds
- Frame delta: 0.05s (20 FPS equivalent)

**Raw Output**:
```
1000 units, 100 ticks in 1.159279625s (11.59 ms/tick)
```

**Metrics**:
| Metric | Value |
|--------|-------|
| Total Units | 1000 |
| Simulation Ticks | 100 |
| Total Wall Time | 1.159s |
| Time per Tick | 11.59 ms |
| Effective FPS | ~86 FPS |

---

### Test: `test_stress_2000_units`

**Configuration**:
- Units: 2000 (1000 Blue + 1000 Red)
- Fixed timestep: 20 Hz (1/20 = 0.05s)
- Game time simulated: 2.5 seconds
- Frame delta: 0.05s (20 FPS equivalent)

**Raw Output**:
```
2000 units, 50 ticks in 913.787458ms (18.26 ms/tick)
```

**Metrics**:
| Metric | Value |
|--------|-------|
| Total Units | 2000 |
| Simulation Ticks | 50 |
| Total Wall Time | 913.8 ms |
| Time per Tick | 18.26 ms |
| Effective FPS | ~55 FPS |

---

## Test Code Reference

### Stress Test Implementation

Location: `sim/src/api.rs` (lines 594-670)

```rust
#[test]
fn test_stress_1000_units() {
    use std::time::Instant;
    
    // Use faster timestep for stress test
    let config = SimConfig {
        fixed_timestep: 1.0 / 20.0, // 20 Hz for faster testing
        ..Default::default()
    };
    let mut sim = SimWorld::with_config(config);
    
    // Spawn 500 Blue squads on left side
    sim.spawn_mass_squads(Faction::Blue, -150.0, 0.0, 500, 200.0, 0);
    
    // Spawn 500 Red squads on right side
    sim.spawn_mass_squads(Faction::Red, 150.0, 0.0, 500, 200.0, 10000);
    
    // Verify counts
    let snapshot = sim.snapshot();
    assert_eq!(snapshot.squads.len(), 1000);
    
    // Benchmark: run for 5 seconds of game time
    let start = Instant::now();
    let game_time = 5.0;
    let frame_dt = 0.05; // 20 FPS
    let frames = (game_time / frame_dt) as usize;
    
    for _ in 0..frames {
        sim.step(frame_dt);
    }
    let elapsed = start.elapsed();
    
    let ticks = sim.current_tick();
    println!("1000 units, {} ticks in {:?} ({:.2} ms/tick)", 
             ticks, elapsed, elapsed.as_millis() as f64 / ticks as f64);
    
    // Should complete in reasonable time (< 30 seconds for debug build)
    assert!(elapsed.as_secs() < 30, "Simulation too slow: {:?}", elapsed);
    
    // Verify simulation ran
    assert!(ticks > 0, "Simulation should have advanced");
}

#[test]
fn test_stress_2000_units() {
    use std::time::Instant;
    
    // Use faster timestep for stress test
    let config = SimConfig {
        fixed_timestep: 1.0 / 20.0, // 20 Hz for faster testing
        ..Default::default()
    };
    let mut sim = SimWorld::with_config(config);
    
    // Spawn 1000 Blue squads
    sim.spawn_mass_squads(Faction::Blue, -200.0, 0.0, 1000, 300.0, 0);
    
    // Spawn 1000 Red squads
    sim.spawn_mass_squads(Faction::Red, 200.0, 0.0, 1000, 300.0, 10000);
    
    assert_eq!(sim.snapshot().squads.len(), 2000);
    
    // Benchmark: run for 2.5 seconds of game time
    let start = Instant::now();
    let game_time = 2.5;
    let frame_dt = 0.05;
    let frames = (game_time / frame_dt) as usize;
    
    for _ in 0..frames {
        sim.step(frame_dt);
    }
    let elapsed = start.elapsed();
    
    let ticks = sim.current_tick();
    println!("2000 units, {} ticks in {:?} ({:.2} ms/tick)", 
             ticks, elapsed, elapsed.as_millis() as f64 / ticks as f64);
    
    // Should complete (may be slower, just verify it works)
    assert!(elapsed.as_secs() < 60, "Simulation too slow: {:?}", elapsed);
}
```

---

## Systems Under Test

Each simulation tick runs the following systems:

| System | Description |
|--------|-------------|
| `spatial_grid_update_system` | Updates spatial partitioning grid |
| `lod_assignment_system` | Assigns LOD based on distance |
| `sector_assignment_system` | Assigns units to combat sectors |
| `activity_flags_system` | Updates activity state flags |
| `threat_awareness_system` | Detects nearby enemies |
| `nearby_friendlies_system` | Finds friendly units for flocking |
| `behavior_state_system` | Updates AI behavior states |
| `flocking_ai_system` | Calculates swarm steering |
| `order_system` | Processes movement orders |
| `movement_system` | Applies velocity to position |
| `combat_system` | Resolves attacks and damage |
| `suppression_system` | Applies/decays suppression |
| `morale_system` | Updates morale state |
| `cover_system` | Calculates cover bonuses |
| `terrain_damage_system` | Processes terrain destruction |
| `destruction_system` | Updates destructible states |

---

## Unit Test Categories

### Spatial Grid Tests (3 tests)
- `test_spatial_grid_insert_query` - Basic insert/query operations
- `test_faction_queries` - Faction-based filtering
- `test_nearest_enemy` - Nearest enemy lookup

### AI System Tests (2 tests)
- `test_behavior_state_transitions` - State machine transitions
- `test_threat_awareness_detects_enemies` - Enemy detection

### Combat System Tests (1 test)
- `test_combat_applies_damage_and_suppression` - Damage/suppression application

### Cover System Tests (2 tests)
- `test_cover_damage_reduction` - Damage reduction in cover
- `test_cover_zones` - Cover zone detection

### Destruction System Tests (2 tests)
- `test_destruction_state_transitions` - Destructible state changes
- `test_terrain_damage_affects_destructibles` - Terrain damage propagation

### Morale System Tests (3 tests)
- `test_morale_decreases_when_suppressed` - Suppression effect on morale
- `test_morale_recovers_when_not_suppressed` - Morale recovery
- `test_rout_when_broken` - Broken morale behavior

### Movement System Tests (1 test)
- `test_movement_applies_velocity` - Position updates from velocity

### Performance System Tests (3 tests)
- `test_lod_assignment` - LOD distance calculation
- `test_sim_tick_lod_scheduling` - LOD tick scheduling
- `test_sector_assignment` - Sector ID assignment

### Serialization Tests (1 test)
- `test_snapshot_roundtrip` - JSON serialization/deserialization

### Suppression System Tests (1 test)
- `test_suppression_decays` - Suppression decay over time

### Terrain System Tests (5 tests)
- `test_terrain_grid_creation` - Grid initialization
- `test_world_to_grid` - Coordinate conversion
- `test_cover_values` - Cover value lookup
- `test_movement_multiplier` - Movement speed modifiers
- `test_crater_application` - Crater terrain modification

### Terrain Damage Tests (1 test)
- `test_terrain_damage_cleared` - Damage event cleanup

### API Tests (7 tests)
- `test_new_world` - World initialization
- `test_default_test_world` - Default test setup
- `test_snapshot_json` - Snapshot generation
- `test_step_advances_tick` - Tick advancement
- `test_spatial_grid_populated` - Grid population
- `test_move_order` - Order processing
- `test_mass_spawn_and_step` - Mass spawning

### Stress Tests (2 tests)
- `test_stress_1000_units` - 1000 unit benchmark
- `test_stress_2000_units` - 2000 unit benchmark

---

## Verification Checklist

To verify these results on your machine:

- [ ] Clone repository: `git clone https://github.com/SingSongScreamAlong/Thisbitterground.git`
- [ ] Navigate to sim directory: `cd Thisbitterground/sim`
- [ ] Run full test suite: `cargo test --release`
- [ ] Run stress tests with output: `cargo test test_stress --release -- --nocapture`
- [ ] Compare ms/tick values with this report

**Expected variance**: ±20% depending on hardware and system load.

---

## Hardware Notes

Results will vary based on:
- CPU architecture (ARM vs x86)
- Number of cores
- Clock speed
- System load during test
- Memory bandwidth

The benchmarks were run on Apple Silicon (ARM64) with release optimizations enabled.
