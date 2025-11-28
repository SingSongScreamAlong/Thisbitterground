//! This Bitter Ground - GDExtension bindings
//!
//! Exposes the Rust ECS simulation to Godot 4 via GDExtension.

use godot::prelude::*;

mod sim_bridge;

/// GDExtension entry point.
struct TbgExtension;

#[gdextension]
unsafe impl ExtensionLibrary for TbgExtension {}
