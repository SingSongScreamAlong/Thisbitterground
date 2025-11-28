//! Performance optimization systems.
//!
//! This module contains systems for:
//! - LOD (Level-of-Detail) assignment based on distance
//! - Activity flag updates for skipping idle units
//! - Sector assignment for batched combat
//!
//! ## Parallelism Notes
//! - `lod_assignment_system`: Read-only Position, writes SimLod. Can run in parallel with other read-only systems.
//! - `activity_flags_system`: Reads Velocity/Suppression, writes ActivityFlags. Can run in parallel with non-overlapping systems.
//! - `sector_assignment_system`: Read-only Position, writes SectorId. Can run in parallel with other read-only systems.

use crate::components::*;
use bevy_ecs::prelude::*;

/// Configuration for simulation performance tuning.
#[derive(Resource, Debug, Clone)]
pub struct SimConfig {
    /// Fixed timestep in seconds (e.g., 1/30 = 0.0333 for 30 Hz).
    pub fixed_timestep: f32,
    /// Size of combat sectors in world units.
    pub sector_size: f32,
    /// Distance threshold for High LOD (full fidelity).
    pub lod_high_distance: f32,
    /// Distance threshold for Medium LOD.
    pub lod_medium_distance: f32,
    /// Number of ticks to remember damage for activity flags.
    pub damage_memory_ticks: u64,
    /// Reference point for LOD calculations (e.g., camera position or frontline).
    pub lod_reference_point: (f32, f32),
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            fixed_timestep: 1.0 / 30.0, // 30 Hz
            sector_size: 40.0,           // 40 unit sectors
            lod_high_distance: 100.0,    // Full fidelity within 100 units
            lod_medium_distance: 200.0,  // Medium fidelity within 200 units
            damage_memory_ticks: 60,     // ~2 seconds at 30 Hz
            lod_reference_point: (0.0, 0.0), // Center of battlefield
        }
    }
}

/// Global simulation tick counter.
/// Increments each fixed update, used for LOD scheduling.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct SimTick(pub u64);

impl SimTick {
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    /// Check if an entity with the given LOD should update this tick.
    #[inline]
    pub fn should_update(&self, lod: SimLod) -> bool {
        lod.should_update(self.0)
    }
}

/// System that assigns LOD based on distance to reference point.
/// 
/// ## Data Access
/// - Reads: Position, SimConfig
/// - Writes: SimLod
/// 
/// Can run in parallel with systems that don't write to SimLod.
pub fn lod_assignment_system(
    config: Res<SimConfig>,
    mut query: Query<(&Position, &mut SimLod)>,
) {
    let (ref_x, ref_y) = config.lod_reference_point;
    let high_sq = config.lod_high_distance * config.lod_high_distance;
    let med_sq = config.lod_medium_distance * config.lod_medium_distance;

    for (pos, mut lod) in query.iter_mut() {
        let dx = pos.x - ref_x;
        let dy = pos.y - ref_y;
        let dist_sq = dx * dx + dy * dy;

        let new_lod = if dist_sq <= high_sq {
            SimLod::High
        } else if dist_sq <= med_sq {
            SimLod::Medium
        } else {
            SimLod::Low
        };

        if *lod != new_lod {
            *lod = new_lod;
        }
    }
}

/// System that updates activity flags based on current state.
/// 
/// ## Data Access
/// - Reads: Velocity, Suppression, SimTick, SimConfig
/// - Writes: ActivityFlags
/// 
/// Can run in parallel with systems that don't write to ActivityFlags.
pub fn activity_flags_system(
    tick: Res<SimTick>,
    config: Res<SimConfig>,
    mut query: Query<(&Velocity, &Suppression, &mut ActivityFlags)>,
) {
    for (vel, suppression, mut flags) in query.iter_mut() {
        // Update movement flag
        flags.is_moving = vel.magnitude() > 0.1;
        
        // Update suppression flag
        flags.is_suppressed = suppression.is_suppressed();
        
        // Update damage memory
        flags.update_damage_status(tick.0, config.damage_memory_ticks);
    }
}

/// System that assigns sector IDs based on position.
/// 
/// ## Data Access
/// - Reads: Position, SimConfig
/// - Writes: SectorId
/// 
/// Can run in parallel with systems that don't write to SectorId.
pub fn sector_assignment_system(
    config: Res<SimConfig>,
    mut query: Query<(&Position, &mut SectorId)>,
) {
    let sector_size = config.sector_size;
    for (pos, mut sector) in query.iter_mut() {
        let new_sector = SectorId::from_position(pos.x, pos.y, sector_size);
        if *sector != new_sector {
            *sector = new_sector;
        }
    }
}

/// Aggregated combat statistics for a sector.
/// Used for batched combat calculations.
#[derive(Debug, Clone, Default)]
pub struct SectorCombatStats {
    /// Total incoming damage to this sector.
    pub incoming_damage: f32,
    /// Total incoming suppression to this sector.
    pub incoming_suppression: f32,
    /// Number of enemy units targeting this sector.
    pub enemy_fire_sources: u32,
    /// Number of friendly units in this sector.
    pub friendly_count: u32,
}

/// Resource holding aggregated sector combat data.
/// Rebuilt each tick by the sector aggregation system.
#[derive(Resource, Debug, Default)]
pub struct SectorCombatData {
    /// Map from (sector_x, sector_y, faction) to combat stats.
    /// Faction: 0 = Blue, 1 = Red
    pub stats: std::collections::HashMap<(i32, i32, u8), SectorCombatStats>,
}

impl SectorCombatData {
    pub fn clear(&mut self) {
        self.stats.clear();
    }

    pub fn get_stats(&self, sector: SectorId, faction: u8) -> Option<&SectorCombatStats> {
        self.stats.get(&(sector.0, sector.1, faction))
    }

    pub fn add_damage(&mut self, sector: SectorId, faction: u8, damage: f32, suppression: f32) {
        let entry = self.stats.entry((sector.0, sector.1, faction)).or_default();
        entry.incoming_damage += damage;
        entry.incoming_suppression += suppression;
        entry.enemy_fire_sources += 1;
    }

    pub fn register_unit(&mut self, sector: SectorId, faction: u8) {
        let entry = self.stats.entry((sector.0, sector.1, faction)).or_default();
        entry.friendly_count += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_assignment() {
        let mut world = World::new();
        world.insert_resource(SimConfig::default());

        // Spawn units at different distances
        world.spawn((Position::new(0.0, 0.0), SimLod::Low));    // At reference point
        world.spawn((Position::new(150.0, 0.0), SimLod::Low));  // Medium distance
        world.spawn((Position::new(300.0, 0.0), SimLod::High)); // Far away

        let mut schedule = Schedule::default();
        schedule.add_systems(lod_assignment_system);
        schedule.run(&mut world);

        let mut query = world.query::<(&Position, &SimLod)>();
        let results: Vec<_> = query.iter(&world).collect();

        // Unit at origin should be High LOD
        let near = results.iter().find(|(p, _)| p.x == 0.0).unwrap();
        assert_eq!(*near.1, SimLod::High);

        // Unit at 150 should be Medium LOD
        let mid = results.iter().find(|(p, _)| p.x == 150.0).unwrap();
        assert_eq!(*mid.1, SimLod::Medium);

        // Unit at 300 should be Low LOD
        let far = results.iter().find(|(p, _)| p.x == 300.0).unwrap();
        assert_eq!(*far.1, SimLod::Low);
    }

    #[test]
    fn test_sim_tick_lod_scheduling() {
        let tick = SimTick(0);
        assert!(tick.should_update(SimLod::High));
        assert!(tick.should_update(SimLod::Medium));
        assert!(tick.should_update(SimLod::Low));

        let tick = SimTick(1);
        assert!(tick.should_update(SimLod::High));
        assert!(!tick.should_update(SimLod::Medium)); // Every 2 ticks
        assert!(!tick.should_update(SimLod::Low));    // Every 4 ticks

        let tick = SimTick(2);
        assert!(tick.should_update(SimLod::High));
        assert!(tick.should_update(SimLod::Medium));
        assert!(!tick.should_update(SimLod::Low));

        let tick = SimTick(4);
        assert!(tick.should_update(SimLod::High));
        assert!(tick.should_update(SimLod::Medium));
        assert!(tick.should_update(SimLod::Low));
    }

    #[test]
    fn test_sector_assignment() {
        let mut world = World::new();
        world.insert_resource(SimConfig {
            sector_size: 40.0,
            ..Default::default()
        });

        world.spawn((Position::new(10.0, 10.0), SectorId::default()));
        world.spawn((Position::new(50.0, 10.0), SectorId::default()));
        world.spawn((Position::new(-30.0, -30.0), SectorId::default()));

        let mut schedule = Schedule::default();
        schedule.add_systems(sector_assignment_system);
        schedule.run(&mut world);

        let mut query = world.query::<(&Position, &SectorId)>();
        let results: Vec<_> = query.iter(&world).collect();

        // (10, 10) -> sector (0, 0)
        let s1 = results.iter().find(|(p, _)| p.x == 10.0).unwrap();
        assert_eq!(*s1.1, SectorId(0, 0));

        // (50, 10) -> sector (1, 0)
        let s2 = results.iter().find(|(p, _)| p.x == 50.0).unwrap();
        assert_eq!(*s2.1, SectorId(1, 0));

        // (-30, -30) -> sector (-1, -1)
        let s3 = results.iter().find(|(p, _)| p.x == -30.0).unwrap();
        assert_eq!(*s3.1, SectorId(-1, -1));
    }
}
