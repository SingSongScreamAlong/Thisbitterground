//! Public API for the simulation.
//!
//! This module provides the main interface for Godot (or any other client)
//! to interact with the simulation.
//!
//! ## Fixed Timestep
//! 
//! The simulation uses a fixed timestep internally (default 30 Hz). When `step(dt)` is called,
//! the simulation accumulates time and runs fixed updates as needed. This ensures deterministic
//! behavior regardless of frame rate.
//!
//! ## Performance Optimizations
//! 
//! - **LOD System**: Units far from the action update less frequently
//! - **Sector Batching**: Combat is aggregated by sector to reduce O(n²) comparisons
//! - **Activity Flags**: Idle units skip expensive computations
//! - **Parallel Systems**: Independent systems run in parallel across CPU cores

use crate::components::*;
use crate::spatial::{SpatialGrid, spatial_grid_update_system};
use crate::systems::*;
use crate::terrain::{TerrainGrid, TerrainSnapshot, Crater};
use crate::world::Snapshot;
use bevy_ecs::prelude::*;

/// The main simulation world container.
///
/// Holds the ECS world and schedule, providing a clean API for:
/// - Initializing the simulation
/// - Stepping the simulation forward
/// - Extracting state snapshots
/// - Issuing commands
pub struct SimWorld {
    world: World,
    schedule: Schedule,
    tick: u64,
    time: f32,
    /// Terrain grid for heightmap and terrain effects.
    terrain: TerrainGrid,
    /// New craters since last snapshot (cleared after snapshot).
    new_craters: Vec<Crater>,
    /// Flag indicating terrain was modified.
    terrain_dirty: bool,
    /// Accumulated time for fixed timestep.
    time_accumulator: f32,
}

impl SimWorld {
    /// Create a new empty simulation world.
    pub fn new() -> Self {
        Self::with_config(SimConfig::default())
    }

    /// Create a new simulation world with custom configuration.
    pub fn with_config(config: SimConfig) -> Self {
        let mut world = World::new();
        
        // Core resources
        world.insert_resource(DeltaTime(config.fixed_timestep));
        world.insert_resource(SpatialGrid::new(20.0)); // 20 unit cells
        
        // Performance resources
        world.insert_resource(SimTick(0));
        world.insert_resource(config);
        world.insert_resource(SectorCombatData::default());

        // Build schedule with parallel system groups
        let mut schedule = Schedule::default();
        
        // Group 1: Spatial/LOD systems (must run first, can run in parallel with each other)
        // These update spatial data structures used by later systems
        schedule.add_systems((
            spatial_grid_update_system,
            lod_assignment_system,
            sector_assignment_system,
            activity_flags_system,
        ));
        
        // Group 2: AI awareness systems (read spatial grid, can run in parallel)
        // These detect threats and nearby friendlies
        schedule.add_systems((
            threat_awareness_system,
            nearby_friendlies_system,
            behavior_state_system,
        ).after(spatial_grid_update_system));
        
        // Group 3: AI decision systems (depend on awareness)
        schedule.add_systems((
            ai_order_system,
            flocking_system,
        ).after(behavior_state_system));
        
        // Group 4: Core simulation (main game logic, sequential for correctness)
        schedule.add_systems((
            order_system,
            movement_system,
            combat_system,
            suppression_decay_system,
            morale_system,
            rout_system,
        ).chain().after(flocking_system));
        
        // Group 5: Environment systems
        schedule.add_systems((
            terrain_damage_to_destructibles_system,
            destruction_state_system,
        ).chain().after(rout_system));

        // Create terrain grid: 200x200 cells, 2 units per cell = 400x400 world units
        let terrain = TerrainGrid::new_with_features(200, 200, 2.0);

        Self {
            world,
            schedule,
            tick: 0,
            time: 0.0,
            terrain,
            new_craters: Vec::new(),
            terrain_dirty: true, // Initial terrain needs to be sent
            time_accumulator: 0.0,
        }
    }

    /// Create a test world with some squads for demonstration.
    pub fn new_default_test_world() -> Self {
        let mut sim = Self::new();
        let sector_size = 40.0; // Default sector size

        // Spawn Blue faction squads on the left (player-controlled, no AI)
        for i in 0..6 {
            let x = -50.0;
            let y = -25.0 + (i as f32) * 10.0;
            sim.world.spawn((
                SquadBundle {
                    squad_id: SquadId(i),
                    faction: Faction::Blue,
                    position: Position::new(x, y),
                    velocity: Velocity::default(),
                    health: Health::new(100.0),
                    stats: SquadStats::default(),
                    morale: Morale::default(),
                    suppression: Suppression::default(),
                    order: Order::Hold,
                },
                // Performance components
                SimLod::default(),
                SectorId::from_position(x, y, sector_size),
                ActivityFlags::default(),
            ));
        }

        // Spawn Red faction squads on the right (AI-controlled)
        for i in 0..6 {
            let x = 50.0;
            let y = -25.0 + (i as f32) * 10.0;
            sim.world.spawn((
                SquadBundle {
                    squad_id: SquadId(100 + i),
                    faction: Faction::Red,
                    position: Position::new(x, y),
                    velocity: Velocity::default(),
                    health: Health::new(100.0),
                    stats: SquadStats::default(),
                    morale: Morale::default(),
                    suppression: Suppression::default(),
                    order: Order::Hold,
                },
                AIBundle::default(),
                // Performance components
                SimLod::default(),
                SectorId::from_position(x, y, sector_size),
                ActivityFlags::default(),
            ));
        }

        // Spawn some trees in forest patches
        let mut tree_id = 0u32;
        for &(cx, cy) in &[(-70.0, -70.0), (70.0, -70.0), (-70.0, 70.0), (70.0, 70.0)] {
            for i in 0..8 {
                let angle = (i as f32 / 8.0) * std::f32::consts::TAU;
                let dist = 5.0 + (i as f32 % 3.0) * 3.0;
                let x = cx + dist * angle.cos();
                let y = cy + dist * angle.sin();
                sim.spawn_tree(tree_id, x, y);
                tree_id += 1;
            }
        }

        // Spawn some buildings
        sim.spawn_building(1000, 0.0, -80.0);
        sim.spawn_building(1001, 0.0, 80.0);
        sim.spawn_building(1002, -80.0, 0.0);
        sim.spawn_building(1003, 80.0, 0.0);

        sim
    }

    /// Step the simulation forward by `dt` seconds.
    /// 
    /// Uses fixed timestep internally - accumulates time and runs fixed updates
    /// as needed. This ensures deterministic behavior regardless of frame rate.
    pub fn step(&mut self, dt: f32) {
        // Get fixed timestep from config
        let fixed_dt = self.world
            .get_resource::<SimConfig>()
            .map(|c| c.fixed_timestep)
            .unwrap_or(1.0 / 30.0);

        // Accumulate time
        self.time_accumulator += dt;

        // Run fixed updates
        while self.time_accumulator >= fixed_dt {
            self.fixed_update(fixed_dt);
            self.time_accumulator -= fixed_dt;
        }

        // Update terrain (crater aging, etc.) - runs every frame
        self.terrain.update(dt);
    }

    /// Run a single fixed timestep update.
    fn fixed_update(&mut self, dt: f32) {
        // Update delta time resource
        if let Some(mut dt_res) = self.world.get_resource_mut::<DeltaTime>() {
            dt_res.0 = dt;
        }

        // Increment simulation tick
        if let Some(mut tick_res) = self.world.get_resource_mut::<SimTick>() {
            tick_res.increment();
        }

        // Run all systems
        self.schedule.run(&mut self.world);
        
        self.tick += 1;
        self.time += dt;
    }

    /// Get a snapshot of the current simulation state.
    pub fn snapshot(&mut self) -> Snapshot {
        let mut snapshot = Snapshot::from_world(&mut self.world, self.tick, self.time);
        
        // Add terrain info
        snapshot.new_craters = self.new_craters.clone();
        snapshot.terrain_dirty = self.terrain_dirty;
        
        // Clear new craters after snapshot
        self.new_craters.clear();
        self.terrain_dirty = false;
        
        snapshot
    }

    /// Get the snapshot as a JSON string.
    pub fn snapshot_json(&mut self) -> String {
        self.snapshot().to_json().unwrap_or_else(|_| "{}".to_string())
    }

    /// Get a full terrain snapshot (for initial load or when terrain_dirty).
    pub fn terrain_snapshot(&self) -> TerrainSnapshot {
        TerrainSnapshot::from_grid(&self.terrain)
    }

    /// Get terrain snapshot as JSON.
    pub fn terrain_snapshot_json(&self) -> String {
        serde_json::to_string(&self.terrain_snapshot()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get the current tick number.
    pub fn current_tick(&self) -> u64 {
        self.tick
    }

    /// Get the elapsed simulation time.
    pub fn current_time(&self) -> f32 {
        self.time
    }

    /// Issue a move order to a squad.
    pub fn order_move(&mut self, squad_id: u32, target_x: f32, target_y: f32) {
        let mut query = self.world.query::<(&SquadId, &mut Order)>();
        for (id, mut order) in query.iter_mut(&mut self.world) {
            if id.0 == squad_id {
                *order = Order::MoveTo { x: target_x, y: target_y };
                break;
            }
        }
    }

    /// Issue an attack-move order to a squad.
    pub fn order_attack_move(&mut self, squad_id: u32, target_x: f32, target_y: f32) {
        let mut query = self.world.query::<(&SquadId, &mut Order)>();
        for (id, mut order) in query.iter_mut(&mut self.world) {
            if id.0 == squad_id {
                *order = Order::AttackMove { x: target_x, y: target_y };
                break;
            }
        }
    }

    /// Issue a hold order to a squad.
    pub fn order_hold(&mut self, squad_id: u32) {
        let mut query = self.world.query::<(&SquadId, &mut Order)>();
        for (id, mut order) in query.iter_mut(&mut self.world) {
            if id.0 == squad_id {
                *order = Order::Hold;
                break;
            }
        }
    }

    /// Issue a retreat order to a squad.
    pub fn order_retreat(&mut self, squad_id: u32) {
        let mut query = self.world.query::<(&SquadId, &mut Order)>();
        for (id, mut order) in query.iter_mut(&mut self.world) {
            if id.0 == squad_id {
                *order = Order::Retreat;
                break;
            }
        }
    }

    /// Spawn a terrain damage event (e.g., from artillery).
    pub fn spawn_crater(&mut self, x: f32, y: f32, radius: f32, depth: f32) {
        // Apply to terrain grid
        self.terrain.apply_crater(x, y, radius, depth);
        
        // Track new crater for snapshot
        self.new_craters.push(Crater { x, y, radius, depth, age: 0.0 });
        self.terrain_dirty = true;
        
        // Also spawn ECS event for other systems
        self.world.spawn(TerrainDamageEvent { x, y, radius, depth });
    }

    /// Spawn an artillery barrage.
    pub fn spawn_barrage(&mut self, center_x: f32, center_y: f32, spread: f32, count: usize) {
        let crater_radius = 3.0 + spread * 0.1;
        let crater_depth = 1.5;
        
        self.terrain.apply_barrage(center_x, center_y, spread, count, crater_radius, crater_depth);
        
        // Track all new craters
        for crater in self.terrain.craters.iter().rev().take(count) {
            self.new_craters.push(*crater);
        }
        self.terrain_dirty = true;
    }

    /// Get movement speed multiplier at a position.
    pub fn get_movement_multiplier(&self, x: f32, y: f32) -> f32 {
        self.terrain.get_movement_multiplier(x, y)
    }

    /// Get cover value at a position.
    pub fn get_cover_at(&self, x: f32, y: f32) -> f32 {
        self.terrain.get_cover_at(x, y)
    }

    /// Get terrain height at a position.
    pub fn get_height_at(&self, x: f32, y: f32) -> f32 {
        self.terrain.get_height_at(x, y)
    }

    /// Get terrain grid reference.
    pub fn terrain(&self) -> &TerrainGrid {
        &self.terrain
    }

    /// Get mutable terrain grid reference.
    pub fn terrain_mut(&mut self) -> &mut TerrainGrid {
        self.terrain_dirty = true;
        &mut self.terrain
    }

    /// Spawn a tree at the given position.
    pub fn spawn_tree(&mut self, id: u32, x: f32, y: f32) {
        self.world.spawn(TreeBundle::new(id, x, y));
    }

    /// Spawn a building at the given position.
    pub fn spawn_building(&mut self, id: u32, x: f32, y: f32) {
        self.world.spawn(BuildingBundle::new(id, x, y));
    }

    /// Damage a destructible by ID.
    pub fn damage_destructible(&mut self, id: u32, damage: f32) {
        let mut query = self.world.query::<(&DestructibleId, &mut DestructibleHealth)>();
        for (dest_id, mut health) in query.iter_mut(&mut self.world) {
            if dest_id.0 == id {
                health.damage(damage);
                break;
            }
        }
    }

    /// Get the number of destructibles.
    pub fn destructible_count(&mut self) -> usize {
        let mut query = self.world.query::<&DestructibleId>();
        query.iter(&self.world).count()
    }

    /// Spawn an AI-controlled squad with performance components.
    pub fn spawn_ai_squad(&mut self, id: u32, faction: Faction, x: f32, y: f32) {
        // Get sector size from config
        let sector_size = self.world
            .get_resource::<SimConfig>()
            .map(|c| c.sector_size)
            .unwrap_or(40.0);

        self.world.spawn((
            SquadBundle {
                squad_id: SquadId(id),
                faction,
                position: Position::new(x, y),
                velocity: Velocity::default(),
                health: Health::new(100.0),
                stats: SquadStats::default(),
                morale: Morale::default(),
                suppression: Suppression::default(),
                order: Order::Hold,
            },
            AIBundle::default(),
            // Performance components
            SimLod::default(),
            SectorId::from_position(x, y, sector_size),
            ActivityFlags::default(),
        ));
    }

    /// Enable AI control for an existing squad.
    pub fn enable_ai(&mut self, squad_id: u32) {
        let mut query = self.world.query::<(Entity, &SquadId)>();
        let entity = query.iter(&self.world)
            .find(|(_, id)| id.0 == squad_id)
            .map(|(e, _)| e);
        
        if let Some(entity) = entity {
            self.world.entity_mut(entity).insert(AIBundle::default());
        }
    }

    /// Disable AI control for a squad.
    pub fn disable_ai(&mut self, squad_id: u32) {
        let mut query = self.world.query::<(Entity, &SquadId)>();
        let entity = query.iter(&self.world)
            .find(|(_, id)| id.0 == squad_id)
            .map(|(e, _)| e);
        
        if let Some(entity) = entity {
            self.world.entity_mut(entity).remove::<AIBundle>();
        }
    }

    /// Spawn multiple AI squads in a formation.
    /// Returns the number of squads spawned.
    pub fn spawn_mass_squads(
        &mut self,
        faction: Faction,
        center_x: f32,
        center_y: f32,
        count: usize,
        spread: f32,
        start_id: u32,
    ) -> usize {
        let cols = (count as f32).sqrt().ceil() as usize;
        let spacing = spread / cols as f32;

        for i in 0..count {
            let row = i / cols;
            let col = i % cols;
            let x = center_x + (col as f32 - cols as f32 / 2.0) * spacing;
            let y = center_y + (row as f32 - (count / cols) as f32 / 2.0) * spacing;
            
            self.spawn_ai_squad(start_id + i as u32, faction, x, y);
        }
        count
    }

    /// Get the spatial grid reference (for debugging/visualization).
    pub fn spatial_grid(&self) -> Option<&SpatialGrid> {
        self.world.get_resource::<SpatialGrid>()
    }

    /// Get direct access to the ECS world (for advanced usage).
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Get mutable access to the ECS world (for advanced usage).
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }
}

impl Default for SimWorld {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_world() {
        let sim = SimWorld::new();
        assert_eq!(sim.current_tick(), 0);
    }

    #[test]
    fn test_default_test_world() {
        let mut sim = SimWorld::new_default_test_world();
        let snapshot = sim.snapshot();
        assert_eq!(snapshot.squads.len(), 12); // 6 Blue + 6 Red
    }

    #[test]
    fn test_step_advances_tick() {
        let mut sim = SimWorld::new();
        sim.step(0.05);
        assert_eq!(sim.current_tick(), 1);
        sim.step(0.05);
        assert_eq!(sim.current_tick(), 2);
    }

    #[test]
    fn test_move_order() {
        let mut sim = SimWorld::new_default_test_world();
        sim.order_move(0, 100.0, 50.0);

        // Step a few times
        for _ in 0..10 {
            sim.step(0.1);
        }

        let snapshot = sim.snapshot();
        let squad = snapshot.squads.iter().find(|s| s.id == 0).unwrap();
        
        // Squad should have moved toward target
        assert!(squad.x > -50.0);
    }

    #[test]
    fn test_snapshot_json() {
        let mut sim = SimWorld::new_default_test_world();
        let json = sim.snapshot_json();
        assert!(json.contains("squads"));
        assert!(json.contains("Blue"));
        assert!(json.contains("Red"));
    }

    #[test]
    fn test_mass_spawn_and_step() {
        let mut sim = SimWorld::new();
        
        // Spawn 100 Blue squads
        sim.spawn_mass_squads(Faction::Blue, -100.0, 0.0, 100, 80.0, 0);
        
        // Spawn 100 Red squads
        sim.spawn_mass_squads(Faction::Red, 100.0, 0.0, 100, 80.0, 1000);
        
        // Verify counts
        let snapshot = sim.snapshot();
        assert_eq!(snapshot.squads.len(), 200);
        
        // Run simulation for ~0.5 seconds of game time
        // With fixed timestep at 30 Hz, this runs ~15 fixed updates
        for _ in 0..10 {
            sim.step(0.05);
        }
        
        // Verify simulation ran (tick count depends on fixed timestep)
        assert!(sim.current_tick() > 0, "Simulation should have advanced");
    }

    #[test]
    fn test_spatial_grid_populated() {
        let mut sim = SimWorld::new();
        sim.spawn_mass_squads(Faction::Blue, 0.0, 0.0, 50, 100.0, 0);
        
        // Step once to populate spatial grid
        sim.step(0.05);
        
        // Check spatial grid has entries
        if let Some(grid) = sim.spatial_grid() {
            assert_eq!(grid.total_count(), 50);
        }
    }

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
        println!("1000 units, {} ticks in {:?} ({:.2} ms/tick)", ticks, elapsed, elapsed.as_millis() as f64 / ticks as f64);
        
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
        println!("2000 units, {} ticks in {:?} ({:.2} ms/tick)", ticks, elapsed, elapsed.as_millis() as f64 / ticks as f64);
        
        // Should complete (may be slower, just verify it works)
        assert!(elapsed.as_secs() < 60, "Simulation too slow: {:?}", elapsed);
    }
}
