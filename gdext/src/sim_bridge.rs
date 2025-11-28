//! SimWorldBridge - Godot class that wraps the Rust simulation.

use godot::prelude::*;
use tbg_sim::SimWorld;

/// Bridge class exposing the Rust simulation to Godot.
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
}
