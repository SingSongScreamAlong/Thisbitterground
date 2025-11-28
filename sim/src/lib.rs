//! This Bitter Ground - Simulation Core
//!
//! A deterministic, fixed-timestep ECS simulation for large-scale war gaming.
//! Uses `bevy_ecs` for the entity-component-system architecture.
//!
//! ## Simulation Rates
//!
//! - **30 Hz (Normal)**: Production rate, recommended for up to ~3000 units.
//! - **20 Hz (Performance)**: Large-scale mode, suitable for up to ~5000 units.
//!
//! See [`SimRate`] and [`SimConfig`] for configuration options.
//!
//! ## Godot Integration
//!
//! The [`godot_bridge`] module provides FFI-friendly interfaces for Godot/GDExtension.

pub mod api;
pub mod components;
pub mod godot_bridge;
pub mod profiler;
pub mod spatial;
pub mod systems;
pub mod terrain;
pub mod world;

pub use components::*;
pub use godot_bridge::snapshot_to_flatbuffer;
pub use profiler::{Profiler, StressProfiler, SectionStats};
pub use spatial::{SpatialGrid, SpatialEntry};
pub use systems::*;
pub use terrain::{TerrainGrid, TerrainCell, TerrainType, TerrainSnapshot, Crater, TerrainResource};
pub use world::Snapshot;
pub use api::SimWorld;
