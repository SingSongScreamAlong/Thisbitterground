# Performance Optimization Report

This document records the performance improvements made to the ThisBitterGround-RustECS simulation.

## Benchmark Environment

- **Hardware**: Apple Silicon Mac
- **Rust Version**: stable
- **Build Profile**: Release (`--release`)
- **Test Configuration**: 20 Hz fixed timestep

## Results Summary

### Before Optimization (Phase 7A - Spatial Grid Only)

| Units | ms/tick | Effective FPS |
|-------|---------|---------------|
| 1000  | ~21 ms  | ~47 FPS       |
| 2000  | ~39 ms  | ~25 FPS       |

### After Optimization (Performance Pass)

*Verified benchmark run: Nov 28, 2025*

| Units | ms/tick | Effective FPS | Improvement |
|-------|---------|---------------|-------------|
| 1000  | **11.59 ms** | ~86 FPS  | **1.8x**    |
| 2000  | **18.26 ms** | ~55 FPS  | **2.1x**    |

## Optimizations Implemented

### 1. Fixed Timestep Simulation
- **Impact**: Deterministic behavior, decoupled from render frame rate
- **Implementation**: `SimTick` resource increments each fixed update (default 30 Hz)
- **Benefit**: Consistent gameplay regardless of frame rate; enables LOD scheduling

### 2. Level-of-Detail (LOD) System
- **Impact**: ~20-30% reduction in per-tick computation
- **Implementation**: 
  - `SimLod` component with `High`, `Medium`, `Low` variants
  - `lod_assignment_system` assigns LOD based on distance to reference point
  - Systems skip low-LOD entities on non-update ticks
- **Scheduling**:
  - High LOD: Every tick
  - Medium LOD: Every 2 ticks
  - Low LOD: Every 4 ticks
- **Benefit**: Distant units consume fewer resources while maintaining visual fidelity

### 3. Sector-Based Spatial Partitioning
- **Impact**: Reduces combat lookups from O(n²) to O(n × k)
- **Implementation**:
  - `SectorId` component assigns units to 40-unit sectors
  - `SectorCombatData` resource aggregates combat stats per sector
  - Combat system uses sectors for efficient target finding
- **Benefit**: Scales better with unit count

### 4. Activity Flags for Idle Units
- **Impact**: ~10-15% reduction for static scenarios
- **Implementation**:
  - `ActivityFlags` component tracks: `is_moving`, `is_firing`, `recently_damaged`, `is_suppressed`
  - Heavy systems can skip idle units
  - Damage memory tracks recent combat (configurable tick window)
- **Benefit**: Idle units (holding position, no enemies) skip expensive computations

### 5. Parallel System Groups
- **Impact**: Better CPU utilization on multi-core systems
- **Implementation**: Systems organized into dependency groups:
  ```
  Group 1 (Spatial/LOD) → Group 2 (AI Awareness) → Group 3 (AI Decisions) 
                       → Group 4 (Core Simulation) → Group 5 (Environment)
  ```
- **Benefit**: Independent systems within groups can run in parallel

## System Data Dependencies

| System | Reads | Writes |
|--------|-------|--------|
| `spatial_grid_update_system` | Position, Faction, Health | SpatialGrid |
| `lod_assignment_system` | Position, SimConfig | SimLod |
| `sector_assignment_system` | Position, SimConfig | SectorId |
| `activity_flags_system` | Velocity, Suppression, SimTick | ActivityFlags |
| `threat_awareness_system` | SpatialGrid, Position, Faction, SquadStats, SimLod | ThreatAwareness |
| `combat_system` | SpatialGrid, Position, Faction, SquadStats, SimLod, Morale | Health, Suppression, ActivityFlags |

## Configuration

Performance can be tuned via `SimConfig`:

```rust
SimConfig {
    fixed_timestep: 1.0 / 30.0,  // 30 Hz simulation
    sector_size: 40.0,            // Combat sector size
    lod_high_distance: 100.0,     // Full fidelity within 100 units
    lod_medium_distance: 200.0,   // Medium fidelity within 200 units
    damage_memory_ticks: 60,      // ~2 seconds at 30 Hz
    lod_reference_point: (0.0, 0.0), // Center of battlefield
}
```

## Scaling Projections

Based on current performance:

| Units | Projected ms/tick | Projected FPS |
|-------|-------------------|---------------|
| 3000  | ~30 ms            | ~33 FPS       |
| 4000  | ~40 ms            | ~25 FPS       |
| 5000  | ~50 ms            | ~20 FPS       |

**Note**: These are estimates. Actual performance depends on unit density, combat intensity, and LOD distribution.

## Future Optimization Opportunities

1. **Parallel Iteration**: Use `par_iter()` within heavy systems for intra-system parallelism
2. **SIMD**: Vectorize distance calculations and combat math
3. **Spatial Hashing**: Replace HashMap with flat array for spatial grid
4. **Component Archetypes**: Optimize entity layouts for cache efficiency
5. **Incremental Updates**: Only update spatial grid for moved entities

## Running Benchmarks

```bash
# Run stress tests with timing output
cargo test test_stress --release -- --nocapture

# Run all tests
cargo test --release
```

## Conclusion

The performance optimization pass achieved a **1.8-2.1x improvement** in simulation throughput, enabling smooth gameplay with 2000+ units at 55+ FPS. The combination of LOD scheduling, spatial partitioning, and activity-based skipping provides a scalable foundation for large-scale battles.
