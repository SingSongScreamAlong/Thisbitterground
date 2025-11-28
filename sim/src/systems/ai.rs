//! AI systems for autonomous squad behavior.
//!
//! Implements swarm/flocking behaviors and tactical decision-making.
//! Uses spatial grid for O(1) neighbor lookups instead of O(n) brute force.
//!
//! ## Performance Optimizations
//! - Uses spatial grid for efficient neighbor queries
//! - Respects LOD scheduling (low-LOD units update less frequently)
//! - Skips idle units where appropriate

use crate::components::*;
use crate::spatial::SpatialGrid;
use crate::systems::movement::DeltaTime;
use crate::systems::performance::SimTick;
use bevy_ecs::prelude::*;

// ============================================================================
// THREAT AWARENESS SYSTEM
// ============================================================================

/// System that updates threat awareness for AI-controlled squads.
/// Uses spatial grid for efficient enemy detection.
/// 
/// ## Data Access
/// - Reads: DeltaTime, SpatialGrid, SimTick, Position, Faction, SquadStats, SimLod
/// - Writes: ThreatAwareness
pub fn threat_awareness_system(
    dt: Res<DeltaTime>,
    grid: Res<SpatialGrid>,
    tick: Option<Res<SimTick>>,
    mut ai_query: Query<(
        Entity,
        &Position,
        &Faction,
        &SquadStats,
        &mut ThreatAwareness,
        Option<&SimLod>,
    ), With<AIControlled>>,
) {
    let delta = dt.0;
    let current_tick = tick.as_ref().map(|t| t.0).unwrap_or(0);

    for (entity, pos, faction, stats, mut threat, lod) in ai_query.iter_mut() {
        // Respect LOD scheduling
        if let Some(lod) = lod {
            if !lod.should_update(current_tick) {
                // Still update time_since_fire for consistency
                threat.time_since_fire += delta * lod.tick_interval() as f32;
                continue;
            }
        }

        threat.clear();
        threat.time_since_fire += delta;

        let my_faction = match faction {
            Faction::Blue => 0u8,
            Faction::Red => 1u8,
        };

        // Use spatial grid for efficient enemy lookup
        let search_radius = stats.fire_range * 2.0;
        let enemies = grid.query_enemies(pos.x, pos.y, search_radius, my_faction);

        let mut closest_dist = f32::MAX;
        let mut enemies_in_range = 0u32;

        for enemy in &enemies {
            if enemy.entity == entity {
                continue;
            }

            let dx = enemy.x - pos.x;
            let dy = enemy.y - pos.y;
            let dist = (dx * dx + dy * dy).sqrt();

            // Track nearest enemy
            if dist < closest_dist {
                closest_dist = dist;
                threat.nearest_enemy = Some((enemy.x, enemy.y));
                threat.nearest_enemy_dist = dist;
            }

            // Count enemies in engagement range
            if dist <= stats.fire_range * 1.5 {
                enemies_in_range += 1;
            }
        }

        threat.enemies_in_range = enemies_in_range;

        // Calculate threat level
        threat.threat_level = if enemies_in_range == 0 {
            0.0
        } else {
            let range_factor = if closest_dist < stats.fire_range * 0.5 {
                1.0
            } else if closest_dist < stats.fire_range {
                0.7
            } else {
                0.3
            };
            let count_factor = (enemies_in_range as f32 / 3.0).min(1.0);
            (range_factor * 0.6 + count_factor * 0.4).min(1.0)
        };
    }
}

// ============================================================================
// NEARBY FRIENDLIES SYSTEM
// ============================================================================

/// System that tracks nearby friendly squads for coordination.
/// Uses spatial grid for efficient neighbor lookup.
pub fn nearby_friendlies_system(
    grid: Res<SpatialGrid>,
    mut ai_query: Query<(
        Entity,
        &SquadId,
        &Position,
        &Faction,
        &FlockingWeights,
        &mut NearbyFriendlies,
    ), With<AIControlled>>,
    velocities: Query<&Velocity>,
) {
    for (entity, _squad_id, pos, faction, weights, mut nearby) in ai_query.iter_mut() {
        nearby.squad_ids.clear();
        nearby.center_of_mass = None;
        nearby.average_velocity = (0.0, 0.0);

        let my_faction = match faction {
            Faction::Blue => 0u8,
            Faction::Red => 1u8,
        };

        // Use spatial grid to find nearby friendlies
        let friendlies = grid.query_friendlies(pos.x, pos.y, weights.neighbor_radius, my_faction);

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        let mut sum_vx = 0.0;
        let mut sum_vy = 0.0;
        let mut count = 0;

        for friendly in &friendlies {
            if friendly.entity == entity {
                continue;
            }

            // Get velocity if available
            let (vx, vy) = if let Ok(vel) = velocities.get(friendly.entity) {
                (vel.vx, vel.vy)
            } else {
                (0.0, 0.0)
            };

            // We don't have squad_id in SpatialEntry, so we skip that tracking
            // nearby.squad_ids.push(...) - would need to query for this
            sum_x += friendly.x;
            sum_y += friendly.y;
            sum_vx += vx;
            sum_vy += vy;
            count += 1;
        }

        if count > 0 {
            nearby.center_of_mass = Some((sum_x / count as f32, sum_y / count as f32));
            nearby.average_velocity = (sum_vx / count as f32, sum_vy / count as f32);
        }
    }
}

// ============================================================================
// FLOCKING BEHAVIOR SYSTEM
// ============================================================================

/// System that applies flocking behaviors to AI velocity.
/// Uses spatial grid for efficient separation calculation.
pub fn flocking_system(
    grid: Res<SpatialGrid>,
    mut ai_query: Query<(
        &Position,
        &Faction,
        &mut Velocity,
        &Order,
        &SquadStats,
        &FlockingWeights,
        &NearbyFriendlies,
        &ThreatAwareness,
        &Suppression,
        &Morale,
    ), With<AIControlled>>,
) {
    for (pos, _faction, mut vel, order, stats, weights, nearby, threat, suppression, morale) in ai_query.iter_mut() {
        // Skip if pinned or broken
        if suppression.is_pinned() || morale.is_broken() {
            vel.vx = 0.0;
            vel.vy = 0.0;
            continue;
        }

        let mut steering_x = 0.0;
        let mut steering_y = 0.0;

        // 1. Goal seeking (from current order)
        let (goal_x, goal_y) = match order {
            Order::MoveTo { x, y } | Order::AttackMove { x, y } => (*x, *y),
            Order::Hold => (pos.x, pos.y),
            Order::Retreat => {
                // Move away from nearest enemy
                if let Some((ex, ey)) = threat.nearest_enemy {
                    let dx = pos.x - ex;
                    let dy = pos.y - ey;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                    (pos.x + dx / dist * 50.0, pos.y + dy / dist * 50.0)
                } else {
                    (pos.x, pos.y)
                }
            }
        };

        let goal_dx = goal_x - pos.x;
        let goal_dy = goal_y - pos.y;
        let goal_dist = (goal_dx * goal_dx + goal_dy * goal_dy).sqrt();

        if goal_dist > 1.0 {
            steering_x += (goal_dx / goal_dist) * weights.goal_seeking;
            steering_y += (goal_dy / goal_dist) * weights.goal_seeking;
        }

        // 2. Cohesion (move toward center of nearby friendlies)
        if let Some((cx, cy)) = nearby.center_of_mass {
            let coh_dx = cx - pos.x;
            let coh_dy = cy - pos.y;
            let coh_dist = (coh_dx * coh_dx + coh_dy * coh_dy).sqrt();
            if coh_dist > 1.0 {
                steering_x += (coh_dx / coh_dist) * weights.cohesion;
                steering_y += (coh_dy / coh_dist) * weights.cohesion;
            }
        }

        // 3. Alignment (match velocity of nearby friendlies)
        let (avg_vx, avg_vy) = nearby.average_velocity;
        let avg_speed = (avg_vx * avg_vx + avg_vy * avg_vy).sqrt();
        if avg_speed > 0.1 {
            steering_x += (avg_vx / avg_speed) * weights.alignment;
            steering_y += (avg_vy / avg_speed) * weights.alignment;
        }

        // 4. Separation (avoid crowding) - use spatial grid
        let mut sep_x = 0.0;
        let mut sep_y = 0.0;
        let mut sep_count = 0;

        // Query all nearby units for separation (both factions)
        let nearby_all = grid.query_radius(pos.x, pos.y, weights.separation_radius);
        for other in &nearby_all {
            let dx = pos.x - other.x;
            let dy = pos.y - other.y;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.1 && dist < weights.separation_radius {
                // Push away, stronger when closer
                let strength = 1.0 - (dist / weights.separation_radius);
                sep_x += (dx / dist) * strength;
                sep_y += (dy / dist) * strength;
                sep_count += 1;
            }
        }

        if sep_count > 0 {
            steering_x += (sep_x / sep_count as f32) * weights.separation;
            steering_y += (sep_y / sep_count as f32) * weights.separation;
        }

        // 5. Threat avoidance
        if let Some((ex, ey)) = threat.nearest_enemy {
            if threat.threat_level > 0.3 {
                let avoid_dx = pos.x - ex;
                let avoid_dy = pos.y - ey;
                let avoid_dist = (avoid_dx * avoid_dx + avoid_dy * avoid_dy).sqrt().max(0.1);
                let avoid_strength = threat.threat_level * weights.threat_avoidance;
                steering_x += (avoid_dx / avoid_dist) * avoid_strength * 0.3;
                steering_y += (avoid_dy / avoid_dist) * avoid_strength * 0.3;
            }
        }

        // Normalize and apply speed
        let steer_mag = (steering_x * steering_x + steering_y * steering_y).sqrt();
        if steer_mag > 0.1 {
            let speed = stats.speed;
            vel.vx = (steering_x / steer_mag) * speed;
            vel.vy = (steering_y / steer_mag) * speed;
        } else if goal_dist < 1.0 {
            vel.vx = 0.0;
            vel.vy = 0.0;
        }
    }
}

// ============================================================================
// BEHAVIOR STATE SYSTEM
// ============================================================================

/// System that updates AI behavior state based on situation.
pub fn behavior_state_system(
    mut ai_query: Query<(
        &Order,
        &ThreatAwareness,
        &TacticalPreferences,
        &Suppression,
        &Morale,
        &mut BehaviorState,
    ), With<AIControlled>>,
) {
    for (order, threat, prefs, suppression, morale, mut state) in ai_query.iter_mut() {
        let new_state = determine_behavior_state(order, threat, prefs, suppression, morale);
        if *state != new_state {
            *state = new_state;
        }
    }
}

fn determine_behavior_state(
    order: &Order,
    threat: &ThreatAwareness,
    prefs: &TacticalPreferences,
    suppression: &Suppression,
    morale: &Morale,
) -> BehaviorState {
    // Broken morale = retreating
    if morale.is_broken() {
        return BehaviorState::Retreating;
    }

    // Pinned = taking cover
    if suppression.is_pinned() {
        return BehaviorState::TakingCover;
    }

    // High threat + low aggression = taking cover
    if threat.threat_level > 0.7 && prefs.aggression < 0.5 {
        return BehaviorState::TakingCover;
    }

    // Under fire with cover-seeking preference
    if threat.is_under_fire() && prefs.cover_seeking > 0.5 && suppression.is_suppressed() {
        return BehaviorState::TakingCover;
    }

    // Retreat order
    if matches!(order, Order::Retreat) {
        return BehaviorState::Retreating;
    }

    // Enemies in range = engaging
    if threat.enemies_in_range > 0 {
        // Check if should flank
        if prefs.flanking_tendency > 0.6 && threat.enemies_in_range <= 2 {
            return BehaviorState::Flanking;
        }
        return BehaviorState::Engaging;
    }

    // Moving toward objective
    if matches!(order, Order::MoveTo { .. } | Order::AttackMove { .. }) {
        return BehaviorState::Advancing;
    }

    // Default
    BehaviorState::Idle
}

// ============================================================================
// AI ORDER GENERATION SYSTEM
// ============================================================================

/// System that generates orders for AI squads based on behavior state.
pub fn ai_order_system(
    mut ai_query: Query<(
        &Position,
        &BehaviorState,
        &ThreatAwareness,
        &TacticalPreferences,
        &NearbyFriendlies,
        &mut Order,
    ), With<AIControlled>>,
) {
    for (pos, state, threat, prefs, nearby, mut order) in ai_query.iter_mut() {
        // Only generate orders for certain states
        match state {
            BehaviorState::Retreating => {
                // Move away from threat
                if let Some((ex, ey)) = threat.nearest_enemy {
                    let dx = pos.x - ex;
                    let dy = pos.y - ey;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                    let retreat_dist = 40.0;
                    *order = Order::MoveTo {
                        x: pos.x + (dx / dist) * retreat_dist,
                        y: pos.y + (dy / dist) * retreat_dist,
                    };
                }
            }
            BehaviorState::Flanking => {
                // Move perpendicular to enemy
                if let Some((ex, ey)) = threat.nearest_enemy {
                    let dx = ex - pos.x;
                    let dy = ey - pos.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);
                    // Perpendicular direction (rotate 90 degrees)
                    let perp_x = -dy / dist;
                    let perp_y = dx / dist;
                    let flank_dist = 25.0;
                    *order = Order::AttackMove {
                        x: pos.x + perp_x * flank_dist + (dx / dist) * 10.0,
                        y: pos.y + perp_y * flank_dist + (dy / dist) * 10.0,
                    };
                }
            }
            BehaviorState::Regrouping => {
                // Move toward center of friendlies
                if let Some((cx, cy)) = nearby.center_of_mass {
                    *order = Order::MoveTo { x: cx, y: cy };
                }
            }
            BehaviorState::Engaging => {
                // Attack-move toward enemy if aggressive
                if prefs.aggression > 0.6 {
                    if let Some((ex, ey)) = threat.nearest_enemy {
                        *order = Order::AttackMove { x: ex, y: ey };
                    }
                }
                // Otherwise hold and fire
            }
            _ => {
                // Keep current order
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spatial::spatial_grid_update_system;

    #[test]
    fn test_threat_awareness_detects_enemies() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(0.1));
        world.insert_resource(SpatialGrid::new(20.0));

        // Spawn AI-controlled Blue squad
        world.spawn((
            SquadId(1),
            Faction::Blue,
            Position::new(0.0, 0.0),
            SquadStats::default(),
            Health::new(100.0),
            ThreatAwareness::default(),
            AIControlled,
        ));

        // Spawn enemy Red squad nearby
        world.spawn((
            SquadId(2),
            Faction::Red,
            Position::new(30.0, 0.0),
            Health::new(100.0),
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems((spatial_grid_update_system, threat_awareness_system).chain());
        schedule.run(&mut world);

        // Check threat awareness updated
        let mut query = world.query::<&ThreatAwareness>();
        let threat = query.single(&world);
        assert!(threat.nearest_enemy.is_some());
        assert!(threat.enemies_in_range > 0);
    }

    #[test]
    fn test_behavior_state_transitions() {
        // Test retreating when morale broken
        let state = determine_behavior_state(
            &Order::Hold,
            &ThreatAwareness::default(),
            &TacticalPreferences::default(),
            &Suppression::default(),
            &Morale::new(0.1), // Broken
        );
        assert_eq!(state, BehaviorState::Retreating);

        // Test engaging when enemies in range
        let mut threat = ThreatAwareness::default();
        threat.enemies_in_range = 2;
        let state = determine_behavior_state(
            &Order::Hold,
            &threat,
            &TacticalPreferences::default(),
            &Suppression::default(),
            &Morale::default(),
        );
        assert_eq!(state, BehaviorState::Engaging);
    }
}
