//! Combat system - handles firing and damage between squads.
//! 
//! ## Performance Optimizations
//! - Uses spatial grid for O(k) enemy detection instead of O(n)
//! - Respects LOD: Low-LOD units update less frequently
//! - Skips idle units that aren't firing
//! - Updates activity flags for damage tracking
//!
//! ## Complexity Analysis
//! 
//! The combat system has two phases:
//! 
//! 1. **Gather Phase** - O(n × k) where n = attackers, k = avg enemies per query
//!    - For each attacker, query spatial grid for enemies in range
//!    - Calculate damage/suppression for best target
//!    - This is the EXPENSIVE part and is parallelizable
//!
//! 2. **Apply Phase** - O(n + m) where n = entities, m = damage events
//!    - Apply accumulated damage and suppression
//!    - Update activity flags
//!    - Must be sequential to avoid race conditions
//!
//! ## Parallelization Strategy
//! 
//! The gather phase can be parallelized because:
//! - Each attacker's calculation is independent
//! - Only reads from spatial grid (immutable)
//! - Writes to thread-local CombatResults
//! 
//! Results are merged and applied sequentially in the apply phase.
//!
//! ## Parallel Feature
//! 
//! When compiled with `--features parallel`, the gather phase uses rayon
//! for internal parallel iteration, processing attackers across multiple threads.

use crate::components::*;
use crate::spatial::SpatialGrid;
use crate::systems::movement::DeltaTime;
use crate::systems::performance::SimTick;
use crate::terrain::TerrainResource;
use bevy_ecs::prelude::*;
use std::collections::HashMap;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Combat configuration constants.
const BASE_HIT_CHANCE: f32 = 0.15;
const SUPPRESSION_PER_HIT: f32 = 0.2;
const DAMAGE_PER_HIT: f32 = 8.0;
const RANGE_FALLOFF_START: f32 = 0.5; // Start accuracy falloff at 50% of max range
const MAX_COVER_REDUCTION: f32 = 0.7; // Cover can reduce damage by up to 70%

/// Collected combat results to apply after iteration.
/// 
/// This structure collects damage/suppression intents during the gather phase
/// and applies them atomically in the apply phase.
#[derive(Default, Clone)]
pub struct CombatResults {
    pub damage: HashMap<Entity, f32>,
    pub suppression: HashMap<Entity, f32>,
    /// Entities that fired this tick (for activity tracking)
    pub fired: Vec<Entity>,
}

impl CombatResults {
    /// Merge another CombatResults into this one.
    pub fn merge(&mut self, other: CombatResults) {
        for (entity, dmg) in other.damage {
            *self.damage.entry(entity).or_insert(0.0) += dmg;
        }
        for (entity, sup) in other.suppression {
            *self.suppression.entry(entity).or_insert(0.0) += sup;
        }
        self.fired.extend(other.fired);
    }
}

/// Resource to store pending combat results between gather and apply phases.
/// Used when combat is split into two systems for better parallelization.
#[derive(Resource, Default)]
pub struct PendingCombatResults(pub CombatResults);

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

// ============================================================================
// SPLIT GATHER/APPLY SYSTEMS FOR PARALLELIZATION
// ============================================================================

/// Attacker data extracted for the gather phase.
/// This allows the gather phase to be read-only on entities.
#[derive(Clone)]
struct AttackerData {
    entity: Entity,
    faction: u8,
    x: f32,
    y: f32,
    fire_range: f32,
    accuracy: f32,
    size: u32,
    suppression: f32,
    morale: f32,
    lod_multiplier: f32,
}

/// Combat gather system - computes damage intents without applying them.
/// 
/// ## Complexity: O(n × k) where n = attackers, k = avg enemies per query
/// 
/// ## Data Access (READ-ONLY on entities)
/// - Reads: DeltaTime, SpatialGrid, SimTick, TerrainResource
/// - Reads: Position, Faction, SquadStats, Health, Suppression, Morale, SimLod
/// - Writes: PendingCombatResults (resource only)
/// 
/// This system can run in parallel with other read-only systems because
/// it only writes to a dedicated resource.
pub fn combat_gather_system(
    dt: Res<DeltaTime>,
    grid: Res<SpatialGrid>,
    tick: Option<Res<SimTick>>,
    terrain: Option<Res<TerrainResource>>,
    mut pending: ResMut<PendingCombatResults>,
    query: Query<(
        Entity,
        &Faction,
        &Position,
        &SquadStats,
        &Health,
        &Suppression,
        &Morale,
        Option<&SimLod>,
    )>,
) {
    let delta = dt.0;
    let current_tick = tick.as_ref().map(|t| t.0).unwrap_or(0);
    
    // Clear previous results
    pending.0 = CombatResults::default();

    // GATHER PHASE: Collect attacker data (read-only iteration)
    // Complexity: O(n) where n = total entities
    let attackers: Vec<AttackerData> = query.iter()
        .filter(|(_, _, _, _, health, suppression, morale, lod)| {
            if !health.is_alive() || suppression.value >= 1.0 || morale.value < 0.2 {
                return false;
            }
            if let Some(lod) = lod {
                if !lod.should_update(current_tick) {
                    return false;
                }
            }
            true
        })
        .map(|(entity, faction, pos, stats, _, suppression, morale, lod)| {
            AttackerData {
                entity,
                faction: match faction { Faction::Blue => 0, Faction::Red => 1 },
                x: pos.x,
                y: pos.y,
                fire_range: stats.fire_range,
                accuracy: stats.accuracy,
                size: stats.size,
                suppression: suppression.value,
                morale: morale.value,
                lod_multiplier: lod.map(|l| l.tick_interval() as f32).unwrap_or(1.0),
            }
        })
        .collect();

    // COMPUTE PHASE: Calculate combat interactions
    // Complexity: O(n × k) where n = attackers, k = avg enemies per spatial query
    // This is the expensive part that benefits from parallelization
    
    #[cfg(feature = "parallel")]
    {
        // PARALLEL MODE: Process attackers in parallel using rayon
        // Each thread computes its own CombatResults, then we merge them
        let partial_results: Vec<CombatResults> = attackers
            .par_iter()
            .map(|attacker| {
                compute_attacker_combat(attacker, &grid, terrain.as_ref().map(|t| t.as_ref()), delta)
            })
            .collect();
        
        // Merge all partial results
        for partial in partial_results {
            pending.0.merge(partial);
        }
    }
    
    #[cfg(not(feature = "parallel"))]
    {
        // SEQUENTIAL MODE: Process attackers one by one
        for attacker in &attackers {
            let result = compute_attacker_combat(attacker, &grid, terrain.as_ref().map(|t| t.as_ref()), delta);
            pending.0.merge(result);
        }
    }
}

/// Compute combat for a single attacker. Returns partial CombatResults.
/// This function is pure and can be called in parallel.
fn compute_attacker_combat(
    attacker: &AttackerData,
    grid: &SpatialGrid,
    terrain: Option<&TerrainResource>,
    delta: f32,
) -> CombatResults {
    let mut result = CombatResults::default();
    
    // Spatial query: O(k) where k = enemies in range
    let enemies = grid.query_enemies(attacker.x, attacker.y, attacker.fire_range, attacker.faction);

    // Find best target: O(k)
    let mut best_target: Option<(Entity, f32, f32)> = None;
    for enemy in &enemies {
        let dx = enemy.x - attacker.x;
        let dy = enemy.y - attacker.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist <= attacker.fire_range {
            let target_cover = terrain
                .map(|t| t.get_cover_at(enemy.x, enemy.y))
                .unwrap_or(0.0);

            if best_target.is_none() || dist < best_target.unwrap().1 {
                best_target = Some((enemy.entity, dist, target_cover));
            }
        }
    }

    // Calculate damage if target found
    if let Some((target_entity, dist, target_cover)) = best_target {
        result.fired.push(attacker.entity);

        let range_factor = if dist > attacker.fire_range * RANGE_FALLOFF_START {
            1.0 - ((dist - attacker.fire_range * RANGE_FALLOFF_START) 
                   / (attacker.fire_range * (1.0 - RANGE_FALLOFF_START)))
        } else {
            1.0
        };

        let suppression_penalty = 1.0 - (attacker.suppression * 0.5).min(0.8);
        let morale_factor = 0.5 + attacker.morale * 0.5;
        let effective_accuracy = attacker.accuracy * range_factor * suppression_penalty * morale_factor;

        let shots = attacker.size as f32 * delta * 2.0 * attacker.lod_multiplier;
        let hits = shots * effective_accuracy * BASE_HIT_CHANCE;

        let cover_reduction = 1.0 - (target_cover * MAX_COVER_REDUCTION);
        let final_damage = hits * DAMAGE_PER_HIT * cover_reduction;

        *result.damage.entry(target_entity).or_insert(0.0) += final_damage;
        *result.suppression.entry(target_entity).or_insert(0.0) += 
            hits * SUPPRESSION_PER_HIT * (1.0 - target_cover * 0.3);
    }
    
    result
}

/// Combat apply system - applies pending damage and suppression.
/// 
/// ## Complexity: O(n + m) where n = entities, m = damage events
/// 
/// ## Data Access
/// - Reads: SimTick, PendingCombatResults
/// - Writes: Health, Suppression, ActivityFlags
/// 
/// This system must run after combat_gather_system and should be sequential.
pub fn combat_apply_system(
    tick: Option<Res<SimTick>>,
    pending: Res<PendingCombatResults>,
    mut query: Query<(Entity, &mut Health, &mut Suppression)>,
    mut activity_query: Query<&mut ActivityFlags>,
) {
    let current_tick = tick.as_ref().map(|t| t.0).unwrap_or(0);
    let results = &pending.0;

    // Apply damage and suppression: O(n) iteration, O(1) lookup per entity
    for (entity, mut health, mut suppression) in query.iter_mut() {
        if let Some(&dmg) = results.damage.get(&entity) {
            health.damage(dmg);
            if let Ok(mut flags) = activity_query.get_mut(entity) {
                flags.mark_damaged(current_tick);
            }
        }
        if let Some(&sup) = results.suppression.get(&entity) {
            suppression.add(sup);
        }
    }

    // Update firing flags: O(m) where m = entities that fired
    for entity in &results.fired {
        if let Ok(mut flags) = activity_query.get_mut(*entity) {
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
