//! ECS Systems for This Bitter Ground simulation.
//!
//! Systems contain the game logic that operates on components.
//!
//! ## System Parallelism
//! 
//! Systems are organized into groups that can run in parallel:
//! 
//! **Group 1 (Spatial/LOD)** - Run first, update spatial data:
//! - `spatial_grid_update_system` - Rebuilds spatial grid
//! - `lod_assignment_system` - Assigns LOD based on distance
//! - `sector_assignment_system` - Assigns combat sectors
//! - `activity_flags_system` - Updates activity flags
//! 
//! **Group 2 (AI)** - Can run in parallel, read spatial grid:
//! - `threat_awareness_system` - Detects enemies (reads SpatialGrid)
//! - `nearby_friendlies_system` - Finds friendlies (reads SpatialGrid)
//! - `behavior_state_system` - Updates AI state
//! 
//! **Group 3 (AI Orders)** - Depends on Group 2:
//! - `ai_order_system` - Generates orders from AI state
//! - `flocking_system` - Applies flocking behavior
//! 
//! **Group 4 (Core Simulation)** - Main game logic:
//! - `order_system` - Processes orders into velocity
//! - `movement_system` - Applies velocity to position
//! - `combat_system` - Handles firing and damage
//! - `suppression_decay_system` - Decays suppression over time
//! - `morale_system` - Updates morale
//! - `rout_system` - Handles broken units
//! 
//! **Group 5 (Environment)** - Terrain and destructibles:
//! - `terrain_damage_to_destructibles_system`
//! - `destruction_state_system`

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
