//! Combat system - handles firing and damage between squads.
//! 
//! ## Performance Optimizations
//! - Uses spatial grid for O(k) enemy detection instead of O(n)
//! - Respects LOD: Low-LOD units update less frequently
//! - Skips idle units that aren't firing
//! - Updates activity flags for damage tracking

use crate::components::*;
use crate::spatial::SpatialGrid;
use crate::systems::movement::DeltaTime;
use crate::systems::performance::SimTick;
use crate::terrain::TerrainResource;
use bevy_ecs::prelude::*;
use std::collections::HashMap;

/// Combat configuration constants.
const BASE_HIT_CHANCE: f32 = 0.15;
const SUPPRESSION_PER_HIT: f32 = 0.2;
const DAMAGE_PER_HIT: f32 = 8.0;
const RANGE_FALLOFF_START: f32 = 0.5; // Start accuracy falloff at 50% of max range
const MAX_COVER_REDUCTION: f32 = 0.7; // Cover can reduce damage by up to 70%

/// Collected combat results to apply after iteration.
#[derive(Default)]
struct CombatResults {
    damage: HashMap<Entity, f32>,
    suppression: HashMap<Entity, f32>,
    /// Entities that fired this tick (for activity tracking)
    fired: Vec<Entity>,
}

/// System that processes combat between opposing squads.
/// 
/// ## Data Access
/// - Reads: DeltaTime, SpatialGrid, SimTick, TerrainResource, Position, Faction, SquadStats, Morale, SimLod
/// - Writes: Health, Suppression, ActivityFlags
/// 
/// ## Performance
/// - Uses spatial grid for efficient enemy detection
/// - Respects LOD scheduling (low-LOD units fire less often)
/// - Updates activity flags for damage tracking
pub fn combat_system(
    dt: Res<DeltaTime>,
    grid: Res<SpatialGrid>,
    tick: Option<Res<SimTick>>,
    terrain: Option<Res<TerrainResource>>,
    mut query: Query<(
        Entity,
        &SquadId,
        &Faction,
        &Position,
        &SquadStats,
        &mut Health,
        &mut Suppression,
        &Morale,
        Option<&SimLod>,
    )>,
    mut activity_query: Query<&mut ActivityFlags>,
) {
    let delta = dt.0;
    let current_tick = tick.as_ref().map(|t| t.0).unwrap_or(0);
    let mut results = CombatResults::default();

    // Collect attacker data first to avoid borrow issues
    let attackers: Vec<_> = query.iter()
        .filter(|(_, _, _, _, _, health, suppression, morale, lod)| {
            // Must be alive and able to fire
            if !health.is_alive() || suppression.value >= 1.0 || morale.value < 0.2 {
                return false;
            }
            // Respect LOD scheduling
            if let Some(lod) = lod {
                if !lod.should_update(current_tick) {
                    return false;
                }
            }
            true
        })
        .map(|(entity, _, faction, pos, stats, _, suppression, morale, lod)| {
            let my_faction = match faction {
                Faction::Blue => 0u8,
                Faction::Red => 1u8,
            };
            // LOD affects fire rate - low LOD fires less but with accumulated damage
            let lod_multiplier = lod.map(|l| l.tick_interval() as f32).unwrap_or(1.0);
            (entity, my_faction, pos.x, pos.y, stats.fire_range, stats.accuracy, stats.size, suppression.value, morale.value, lod_multiplier)
        })
        .collect();

    // Process each attacker
    for (entity, my_faction, x, y, fire_range, accuracy, size, suppression_val, morale_val, lod_mult) in &attackers {
        // Use spatial grid to find enemies in range
        let enemies = grid.query_enemies(*x, *y, *fire_range, *my_faction);

        // Find closest enemy
        let mut best_target: Option<(Entity, f32, f32)> = None; // (entity, dist, cover)
        for enemy in &enemies {
            let dx = enemy.x - x;
            let dy = enemy.y - y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist <= *fire_range {
                // Get cover at target position
                let target_cover = terrain.as_ref()
                    .map(|t| t.get_cover_at(enemy.x, enemy.y))
                    .unwrap_or(0.0);

                if best_target.is_none() || dist < best_target.unwrap().1 {
                    best_target = Some((enemy.entity, dist, target_cover));
                }
            }
        }

        // Fire at target if found
        if let Some((target_entity, dist, target_cover)) = best_target {
            // Mark this entity as firing
            results.fired.push(*entity);

            // Calculate hit chance with modifiers
            let range_factor = if dist > fire_range * RANGE_FALLOFF_START {
                1.0 - ((dist - fire_range * RANGE_FALLOFF_START) / (fire_range * (1.0 - RANGE_FALLOFF_START)))
            } else {
                1.0
            };

            // Suppression reduces accuracy
            let suppression_penalty = 1.0 - (suppression_val * 0.5).min(0.8);

            // Morale affects accuracy
            let morale_factor = 0.5 + morale_val * 0.5;

            let effective_accuracy =
                accuracy * range_factor * suppression_penalty * morale_factor;

            // Each soldier in squad fires
            // LOD multiplier compensates for skipped ticks
            let shots = *size as f32 * delta * 2.0 * lod_mult;
            let hits = shots * effective_accuracy * BASE_HIT_CHANCE;

            // Apply cover damage reduction to target
            let cover_reduction = 1.0 - (target_cover * MAX_COVER_REDUCTION);
            let final_damage = hits * DAMAGE_PER_HIT * cover_reduction;

            // Apply damage and suppression (suppression is less affected by cover)
            *results.damage.entry(target_entity).or_insert(0.0) += final_damage;
            *results.suppression.entry(target_entity).or_insert(0.0) += hits * SUPPRESSION_PER_HIT * (1.0 - target_cover * 0.3);
        }
    }

    // Apply accumulated results
    for (entity, _, _, _, _, mut health, mut suppression, _, _) in query.iter_mut() {
        if let Some(&dmg) = results.damage.get(&entity) {
            health.damage(dmg);
            // Mark as recently damaged
            if let Ok(mut flags) = activity_query.get_mut(entity) {
                flags.mark_damaged(current_tick);
            }
        }
        if let Some(&sup) = results.suppression.get(&entity) {
            suppression.add(sup);
        }
    }

    // Update firing flags
    for entity in results.fired {
        if let Ok(mut flags) = activity_query.get_mut(entity) {
            flags.is_firing = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::spatial_grid_update_system;

    #[test]
    fn test_combat_applies_damage_and_suppression() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(0.1));
        world.insert_resource(SpatialGrid::new(20.0));

        // Spawn two opposing squads in range
        world.spawn((
            SquadId(1),
            Faction::Blue,
            Position::new(0.0, 0.0),
            SquadStats::default(),
            Health::new(100.0),
            Suppression::default(),
            Morale::default(),
        ));

        world.spawn((
            SquadId(2),
            Faction::Red,
            Position::new(30.0, 0.0), // Within default 60 unit range
            SquadStats::default(),
            Health::new(100.0),
            Suppression::default(),
            Morale::default(),
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems((spatial_grid_update_system, combat_system).chain());
        
        // Run multiple ticks to accumulate damage
        for _ in 0..10 {
            schedule.run(&mut world);
        }

        // Both squads should have taken damage and suppression
        let mut query = world.query::<(&Health, &Suppression)>();
        for (health, sup) in query.iter(&world) {
            assert!(health.current < 100.0, "Health should decrease from combat");
            assert!(sup.value > 0.0, "Suppression should increase from combat");
        }
    }
}
