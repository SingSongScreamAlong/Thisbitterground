//! This Bitter Ground - Simulation Core
//!
//! A deterministic, fixed-timestep ECS simulation for large-scale war gaming.
//! Uses `bevy_ecs` for the entity-component-system architecture.

pub mod api;
pub mod components;
pub mod spatial;
pub mod systems;
pub mod terrain;
pub mod world;

pub use components::*;
pub use spatial::{SpatialGrid, SpatialEntry};
pub use systems::*;
pub use terrain::{TerrainGrid, TerrainCell, TerrainType, TerrainSnapshot, Crater, TerrainResource};
pub use world::Snapshot;
pub use api::SimWorld;
