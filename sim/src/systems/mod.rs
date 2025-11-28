//! ECS Systems for This Bitter Ground simulation.
//!
//! Systems contain the game logic that operates on components.
//!
//! ## System Parallelism
//! 
//! Systems are organized into groups that can run in parallel.
//! Within each group, systems can potentially run concurrently if their
//! data access patterns don't conflict.
//! 
//! ### Group 1: Spatial/LOD (Run First)
//! 
//! These systems update spatial data structures used by later systems.
//! They can run in parallel with each other.
//! 
//! | System | Reads | Writes |
//! |--------|-------|--------|
//! | `spatial_grid_update_system` | Position, Faction, Health | SpatialGrid |
//! | `lod_assignment_system` | Position, SimConfig | SimLod |
//! | `sector_assignment_system` | Position, SimConfig | SectorId |
//! | `activity_flags_system` | Velocity, Suppression, SimTick | ActivityFlags |
//! 
//! **Parallelization potential**: HIGH - No conflicting writes.
//! 
//! ### Group 2: AI Awareness (After Group 1)
//! 
//! These systems detect threats and nearby units. They read the spatial grid.
//! 
//! | System | Reads | Writes |
//! |--------|-------|--------|
//! | `threat_awareness_system` | SpatialGrid, Position, Faction, SquadStats, SimLod | ThreatAwareness |
//! | `nearby_friendlies_system` | SpatialGrid, Position, Faction, FlockingWeights | NearbyFriendlies |
//! | `behavior_state_system` | ThreatAwareness, Suppression, Morale, Order | BehaviorState |
//! 
//! **Parallelization potential**: MEDIUM - `behavior_state_system` depends on `threat_awareness_system`.
//! 
//! ### Group 3: AI Decisions (After Group 2)
//! 
//! These systems generate movement decisions based on AI state.
//! 
//! | System | Reads | Writes |
//! |--------|-------|--------|
//! | `ai_order_system` | BehaviorState, ThreatAwareness, Position | Order |
//! | `flocking_system` | NearbyFriendlies, ThreatAwareness, Position, FlockingWeights | Velocity |
//! 
//! **Parallelization potential**: HIGH - Different write targets.
//! 
//! ### Group 4: Core Simulation (After Group 3)
//! 
//! Main game logic. Currently chained for correctness, but some could run in parallel.
//! 
//! | System | Reads | Writes | Notes |
//! |--------|-------|--------|-------|
//! | `order_system` | Order, SquadStats | Velocity | |
//! | `movement_system` | Velocity, Suppression, Morale, TerrainResource | Position | |
//! | `combat_system` | SpatialGrid, Position, Faction, SquadStats, SimLod, Morale | Health, Suppression, ActivityFlags | HEAVIEST |
//! | `suppression_decay_system` | DeltaTime | Suppression | |
//! | `morale_system` | Suppression, NearbyFriendlies | Morale | |
//! | `rout_system` | Morale | Velocity, Order | |
//! 
//! **Parallelization potential**: LOW - Sequential dependencies.
//! **Optimization target**: `combat_system` is the heaviest, consider `par_iter`.
//! 
//! ### Group 5: Environment (After Group 4)
//! 
//! Terrain and destructible updates.
//! 
//! | System | Reads | Writes |
//! |--------|-------|--------|
//! | `terrain_damage_to_destructibles_system` | TerrainDamageEvent | DestructibleHealth |
//! | `destruction_state_system` | DestructibleHealth | DestructibleState |
//! 
//! **Parallelization potential**: HIGH - Different entity types.
//! 
//! ## Next Steps for Parallelization
//! 
//! 1. **Intra-system parallelism**: Use `par_iter()` in `combat_system` for the
//!    attacker loop (collect phase only, apply phase must be sequential).
//! 
//! 2. **Group-level parallelism**: Groups 1, 2, and 5 have high potential for
//!    running systems in parallel within the group.
//! 
//! 3. **Profile first**: Use the `Profiler` utility to identify which systems
//!    are actually the bottleneck before optimizing.

pub mod ai;
pub mod combat;
pub mod cover;
pub mod destruction;
pub mod morale;
pub mod movement;
pub mod performance;
pub mod serialization;
pub mod suppression;
pub mod terrain_damage;

pub use ai::*;
pub use combat::*;
pub use cover::*;
pub use destruction::*;
pub use morale::{morale_system, rout_system};
pub use movement::*;
pub use performance::*;
pub use serialization::*;
pub use suppression::*;
pub use terrain_damage::*;
