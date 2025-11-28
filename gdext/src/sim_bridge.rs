//! SimWorldBridge - Godot class that wraps the Rust simulation.
//!
//! This module provides two Godot classes:
//! - `SimWorldBridge`: Full-featured bridge with JSON-based snapshot API
//! - `RustSimulation`: Lightweight class with efficient flat buffer snapshot API
//!
//! ## Flat Buffer Format (RustSimulation)
//!
//! The `get_snapshot_buffer()` method returns a `PackedFloat32Array` with the following layout:
//!
//! ```text
//! [0] = squad_count
//! For each squad i (offset = 1 + i * 14):
//!   [+0]  id          - Squad ID
//!   [+1]  x           - X position
//!   [+2]  y           - Y position
//!   [+3]  vx          - X velocity
//!   [+4]  vy          - Y velocity
//!   [+5]  faction_id  - 0.0=Blue, 1.0=Red
//!   [+6]  size        - Squad size
//!   [+7]  health      - Current health
//!   [+8]  health_max  - Max health
//!   [+9]  morale      - Morale (0.0-1.0)
//!   [+10] suppression - Suppression (0.0-1.0)
//!   [+11] is_alive    - 1.0=alive, 0.0=dead
//!   [+12] is_routing  - 1.0=routing
//!   [+13] order_type  - 0=Hold, 1=Move, 2=Attack, 3=Retreat
//! ```
//!
//! See `sim/src/godot_bridge.rs` for the authoritative format documentation.

use godot::prelude::*;
use godot::builtin::PackedFloat32Array;
use tbg_sim::SimWorld;
use tbg_sim::godot_bridge::{snapshot_to_flatbuffer, SQUAD_STRIDE, HEADER_SIZE};
use tbg_sim::systems::{SimConfig, SimRate};

/// Bridge class exposing the Rust simulation to Godot.
///
/// This is the full-featured bridge with JSON-based snapshot API.
/// For efficient flat buffer snapshots, use `RustSimulation` instead.
///
/// Usage in GDScript:
/// ```gdscript
/// var bridge = SimWorldBridge.new()
/// bridge.init_world()
/// bridge.step(delta)
/// var json = bridge.get_snapshot_json()
/// ```
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct SimWorldBridge {
    base: Base<RefCounted>,
    sim: Option<SimWorld>,
}

#[godot_api]
impl IRefCounted for SimWorldBridge {
    fn init(base: Base<RefCounted>) -> Self {
        Self { base, sim: None }
    }
}

#[godot_api]
impl SimWorldBridge {
    /// Initialize the simulation world with default test squads.
    #[func]
    fn init_world(&mut self) {
        self.sim = Some(SimWorld::new_default_test_world());
        godot_print!("[SimWorldBridge] World initialized with test squads");
    }

    /// Initialize an empty simulation world.
    #[func]
    fn init_empty_world(&mut self) {
        self.sim = Some(SimWorld::new());
        godot_print!("[SimWorldBridge] Empty world initialized");
    }

    /// Step the simulation forward by delta seconds.
    #[func]
    fn step(&mut self, delta: f64) {
        if let Some(ref mut sim) = self.sim {
            sim.step(delta as f32);
        }
    }

    /// Get the current simulation tick.
    #[func]
    fn get_current_tick(&self) -> i64 {
        self.sim
            .as_ref()
            .map(|s| s.current_tick() as i64)
            .unwrap_or(0)
    }

    /// Get the elapsed simulation time in seconds.
    #[func]
    fn get_current_time(&self) -> f64 {
        self.sim
            .as_ref()
            .map(|s| s.current_time() as f64)
            .unwrap_or(0.0)
    }

    /// Get the simulation state as a JSON string.
    #[func]
    fn get_snapshot_json(&mut self) -> GString {
        match &mut self.sim {
            Some(sim) => GString::from(sim.snapshot_json().as_str()),
            None => GString::from("{}"),
        }
    }

    /// Issue a move order to a squad.
    #[func]
    fn order_move(&mut self, squad_id: i32, target_x: f32, target_y: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.order_move(squad_id as u32, target_x, target_y);
        }
    }

    /// Issue an attack-move order to a squad.
    #[func]
    fn order_attack_move(&mut self, squad_id: i32, target_x: f32, target_y: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.order_attack_move(squad_id as u32, target_x, target_y);
        }
    }

    /// Issue a hold order to a squad.
    #[func]
    fn order_hold(&mut self, squad_id: i32) {
        if let Some(ref mut sim) = self.sim {
            sim.order_hold(squad_id as u32);
        }
    }

    /// Issue a retreat order to a squad.
    #[func]
    fn order_retreat(&mut self, squad_id: i32) {
        if let Some(ref mut sim) = self.sim {
            sim.order_retreat(squad_id as u32);
        }
    }

    /// Spawn a terrain damage event (crater).
    #[func]
    fn spawn_crater(&mut self, x: f32, y: f32, radius: f32, depth: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.spawn_crater(x, y, radius, depth);
        }
    }

    /// Spawn an artillery barrage.
    #[func]
    fn spawn_barrage(&mut self, center_x: f32, center_y: f32, spread: f32, count: i32) {
        if let Some(ref mut sim) = self.sim {
            sim.spawn_barrage(center_x, center_y, spread, count as usize);
        }
    }

    /// Get terrain snapshot as JSON.
    #[func]
    fn get_terrain_json(&self) -> GString {
        match &self.sim {
            Some(sim) => GString::from(sim.terrain_snapshot_json().as_str()),
            None => GString::from("{}"),
        }
    }

    /// Get movement multiplier at a position.
    #[func]
    fn get_movement_multiplier(&self, x: f32, y: f32) -> f32 {
        self.sim
            .as_ref()
            .map(|s| s.get_movement_multiplier(x, y))
            .unwrap_or(1.0)
    }

    /// Get cover value at a position.
    #[func]
    fn get_cover_at(&self, x: f32, y: f32) -> f32 {
        self.sim
            .as_ref()
            .map(|s| s.get_cover_at(x, y))
            .unwrap_or(0.0)
    }

    /// Get terrain height at a position.
    #[func]
    fn get_height_at(&self, x: f32, y: f32) -> f32 {
        self.sim
            .as_ref()
            .map(|s| s.get_height_at(x, y))
            .unwrap_or(0.0)
    }

    /// Check if the simulation is initialized.
    #[func]
    fn is_initialized(&self) -> bool {
        self.sim.is_some()
    }

    /// Get the number of squads in the simulation.
    #[func]
    fn get_squad_count(&mut self) -> i32 {
        match &mut self.sim {
            Some(sim) => sim.snapshot().squads.len() as i32,
            None => 0,
        }
    }

    /// Spawn a tree at the given position.
    #[func]
    fn spawn_tree(&mut self, id: i32, x: f32, y: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.spawn_tree(id as u32, x, y);
        }
    }

    /// Spawn a building at the given position.
    #[func]
    fn spawn_building(&mut self, id: i32, x: f32, y: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.spawn_building(id as u32, x, y);
        }
    }

    /// Damage a destructible by ID.
    #[func]
    fn damage_destructible(&mut self, id: i32, damage: f32) {
        if let Some(ref mut sim) = self.sim {
            sim.damage_destructible(id as u32, damage);
        }
    }

    /// Get the number of destructibles.
    #[func]
    fn get_destructible_count(&mut self) -> i32 {
        match &mut self.sim {
            Some(sim) => sim.destructible_count() as i32,
            None => 0,
        }
    }

    /// Get the simulation state as a flat buffer (PackedFloat32Array).
    ///
    /// This is more efficient than JSON for real-time visualization.
    /// See module documentation for the buffer format.
    ///
    /// Layout:
    /// - `buffer[0]` = squad_count
    /// - For each squad: 14 floats (SQUAD_STRIDE) in documented order
    #[func]
    fn get_snapshot_buffer(&mut self) -> PackedFloat32Array {
        match &mut self.sim {
            Some(sim) => {
                let snapshot = sim.snapshot();
                let buffer = snapshot_to_flatbuffer(&snapshot);
                PackedFloat32Array::from(buffer.as_slice())
            }
            None => PackedFloat32Array::new(),
        }
    }

    /// Get the squad stride constant (number of floats per squad in the buffer).
    #[func]
    fn get_squad_stride(&self) -> i32 {
        SQUAD_STRIDE as i32
    }

    /// Get the header size constant (number of floats before squad data).
    #[func]
    fn get_header_size(&self) -> i32 {
        HEADER_SIZE as i32
    }
}

// ============================================================================
// RustSimulation - Lightweight class with efficient flat buffer API
// ============================================================================

/// Lightweight simulation wrapper optimized for real-time visualization.
///
/// This class owns a `SimWorld` instance and provides an efficient API for:
/// - Fixed-timestep simulation stepping
/// - Flat buffer snapshot extraction (no JSON overhead)
/// - Basic order commands
///
/// ## Fixed Timestep
///
/// The simulation runs at a fixed rate (default 30 Hz). When you call `step(delta)`,
/// the Rust side accumulates time and runs fixed updates as needed. This ensures
/// deterministic behavior regardless of Godot's frame rate.
///
/// ## Snapshot Buffer Format
///
/// `get_snapshot_buffer()` returns a `PackedFloat32Array` with:
/// - `buffer[0]` = squad_count
/// - For each squad `i` at offset `1 + i * SQUAD_STRIDE` (SQUAD_STRIDE = 14):
///   - id, x, y, vx, vy, faction_id, size, health, health_max,
///     morale, suppression, is_alive, is_routing, order_type
///
/// See `sim/src/godot_bridge.rs` for the authoritative format documentation.
///
/// ## Usage in GDScript
///
/// ```gdscript
/// extends Node
///
/// const SQUAD_STRIDE = 14
/// var sim: RustSimulation
///
/// func _ready():
///     sim = RustSimulation.new()
///
/// func _process(delta):
///     sim.step(delta)
///     var buffer = sim.get_snapshot_buffer()
///     var squad_count = int(buffer[0])
///     for i in range(squad_count):
///         var offset = 1 + i * SQUAD_STRIDE
///         var x = buffer[offset + 1]
///         var y = buffer[offset + 2]
///         # Update unit visuals...
/// ```
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct RustSimulation {
    base: Base<RefCounted>,
    /// The underlying simulation world.
    sim: SimWorld,
}

#[godot_api]
impl IRefCounted for RustSimulation {
    /// Initialize with a default 30 Hz simulation.
    fn init(base: Base<RefCounted>) -> Self {
        let config = SimConfig::with_rate(SimRate::Normal30Hz);
        Self {
            base,
            sim: SimWorld::with_config(config),
        }
    }
}

#[godot_api]
impl RustSimulation {
    // ========================================================================
    // SIMULATION CONTROL
    // ========================================================================

    /// Step the simulation forward by `delta` seconds.
    ///
    /// The simulation uses fixed-timestep internally (30 Hz by default).
    /// Time is accumulated and fixed updates run as needed.
    /// Godot just calls this each frame with its delta.
    #[func]
    fn step(&mut self, delta: f64) {
        self.sim.step(delta as f32);
    }

    /// Get the current simulation tick.
    #[func]
    fn get_tick(&self) -> i64 {
        self.sim.current_tick() as i64
    }

    /// Get the elapsed simulation time in seconds.
    #[func]
    fn get_time(&self) -> f64 {
        self.sim.current_time() as f64
    }

    // ========================================================================
    // SNAPSHOT API
    // ========================================================================

    /// Get the simulation state as a flat buffer.
    ///
    /// Returns a `PackedFloat32Array` with the following layout:
    /// - `buffer[0]` = squad_count (as f32)
    /// - For each squad `i` at offset `1 + i * 14`:
    ///   - [+0] id, [+1] x, [+2] y, [+3] vx, [+4] vy
    ///   - [+5] faction_id (0=Blue, 1=Red)
    ///   - [+6] size, [+7] health, [+8] health_max
    ///   - [+9] morale, [+10] suppression
    ///   - [+11] is_alive (1.0/0.0), [+12] is_routing (1.0/0.0)
    ///   - [+13] order_type (0=Hold, 1=Move, 2=Attack, 3=Retreat)
    ///
    /// This is the primary method for extracting simulation state for visualization.
    /// It is more efficient than JSON serialization.
    #[func]
    fn get_snapshot_buffer(&mut self) -> PackedFloat32Array {
        let snapshot = self.sim.snapshot();
        let buffer = snapshot_to_flatbuffer(&snapshot);
        PackedFloat32Array::from(buffer.as_slice())
    }

    /// Get the squad stride constant (14).
    ///
    /// This is the number of f32 values per squad in the snapshot buffer.
    #[func]
    fn get_squad_stride(&self) -> i32 {
        SQUAD_STRIDE as i32
    }

    /// Get the header size constant (1).
    ///
    /// This is the number of f32 values before squad data (just squad_count).
    #[func]
    fn get_header_size(&self) -> i32 {
        HEADER_SIZE as i32
    }

    // ========================================================================
    // ORDER API
    // ========================================================================

    /// Issue a move order to a squad.
    ///
    /// The squad will move to the target position.
    #[func]
    fn issue_move_order(&mut self, squad_id: i32, x: f32, y: f32) {
        self.sim.order_move(squad_id as u32, x, y);
    }

    /// Issue an attack-move order to a squad.
    ///
    /// The squad will move to the target, engaging enemies along the way.
    #[func]
    fn issue_attack_move_order(&mut self, squad_id: i32, x: f32, y: f32) {
        self.sim.order_attack_move(squad_id as u32, x, y);
    }

    /// Issue a hold order to a squad.
    ///
    /// The squad will stop moving and hold position.
    #[func]
    fn issue_hold_order(&mut self, squad_id: i32) {
        self.sim.order_hold(squad_id as u32);
    }

    /// Issue a retreat order to a squad.
    ///
    /// The squad will retreat from combat.
    #[func]
    fn issue_retreat_order(&mut self, squad_id: i32) {
        self.sim.order_retreat(squad_id as u32);
    }

    // ========================================================================
    // SPAWNING API
    // ========================================================================

    /// Spawn an AI-controlled squad.
    ///
    /// - `squad_id`: Unique ID for the squad
    /// - `faction`: 0 = Blue, 1 = Red
    /// - `x`, `y`: Initial position
    #[func]
    fn spawn_squad(&mut self, squad_id: i32, faction: i32, x: f32, y: f32) {
        let faction_enum = if faction == 0 {
            tbg_sim::components::Faction::Blue
        } else {
            tbg_sim::components::Faction::Red
        };
        self.sim.spawn_ai_squad(squad_id as u32, faction_enum, x, y);
    }

    /// Spawn multiple squads in a formation.
    ///
    /// - `faction`: 0 = Blue, 1 = Red
    /// - `center_x`, `center_y`: Center of the formation
    /// - `count`: Number of squads to spawn
    /// - `spread`: Size of the formation area
    /// - `start_id`: Starting squad ID (increments for each squad)
    ///
    /// Returns the number of squads spawned.
    #[func]
    fn spawn_mass_squads(
        &mut self,
        faction: i32,
        center_x: f32,
        center_y: f32,
        count: i32,
        spread: f32,
        start_id: i32,
    ) -> i32 {
        let faction_enum = if faction == 0 {
            tbg_sim::components::Faction::Blue
        } else {
            tbg_sim::components::Faction::Red
        };
        self.sim.spawn_mass_squads(
            faction_enum,
            center_x,
            center_y,
            count as usize,
            spread,
            start_id as u32,
        ) as i32
    }

    /// Get the number of squads in the simulation.
    #[func]
    fn get_squad_count(&mut self) -> i32 {
        self.sim.snapshot().squads.len() as i32
    }
}
