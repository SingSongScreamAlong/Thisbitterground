//! Morale system - handles morale changes based on combat conditions.

use crate::components::*;
use crate::systems::movement::DeltaTime;
use bevy_ecs::prelude::*;

/// Rate at which morale recovers per second (when not suppressed).
const MORALE_RECOVERY_RATE: f32 = 0.02;

/// Rate at which morale decreases when suppressed.
const MORALE_SUPPRESSION_PENALTY: f32 = 0.05;

/// Rate at which morale decreases when pinned.
const MORALE_PINNED_PENALTY: f32 = 0.1;

/// Morale penalty per casualty (health lost).
const MORALE_CASUALTY_RATE: f32 = 0.002;

/// Morale boost from nearby friendly squads.
const MORALE_COHESION_BONUS: f32 = 0.01;

/// System that updates morale based on suppression, health, and nearby units.
pub fn morale_system(
    dt: Res<DeltaTime>,
    mut query: Query<(&mut Morale, &Suppression, &Health, &Position, &Faction)>,
) {
    let delta = dt.0;

    // First pass: collect positions for cohesion calculation
    let squad_positions: Vec<_> = query
        .iter()
        .map(|(_, _, _, pos, faction)| (pos.x, pos.y, *faction))
        .collect();

    // Second pass: update morale
    for (mut morale, suppression, health, pos, faction) in query.iter_mut() {
        // Pinned squads lose morale faster
        if suppression.is_pinned() {
            morale.decrease(MORALE_PINNED_PENALTY * delta);
        } else if suppression.is_suppressed() {
            morale.decrease(MORALE_SUPPRESSION_PENALTY * delta);
        }

        // Low health affects morale (casualties)
        let health_frac = health.fraction();
        if health_frac < 0.75 {
            let casualty_penalty = (1.0 - health_frac) * MORALE_CASUALTY_RATE;
            morale.decrease(casualty_penalty * delta);
        }

        // Very low health causes panic
        if health_frac < 0.25 {
            morale.decrease(0.05 * delta);
        }

        // Count nearby friendly squads for cohesion bonus
        let mut nearby_friendlies = 0;
        for (ox, oy, ofaction) in &squad_positions {
            if ofaction != faction {
                continue;
            }
            let dx = ox - pos.x;
            let dy = oy - pos.y;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 0.1 && dist < 20.0 {
                nearby_friendlies += 1;
            }
        }

        // Recovery when not suppressed and not broken
        if !suppression.is_suppressed() && !morale.is_broken() {
            let cohesion_bonus = (nearby_friendlies as f32) * MORALE_COHESION_BONUS;
            morale.recover((MORALE_RECOVERY_RATE + cohesion_bonus) * delta);
        }

        // Broken squads recover very slowly even when safe
        if morale.is_broken() && !suppression.is_suppressed() && health_frac > 0.5 {
            morale.recover(MORALE_RECOVERY_RATE * 0.1 * delta);
        }
    }
}

/// System that handles squad behavior when morale breaks (rout).
pub fn rout_system(
    mut query: Query<(&Morale, &mut Order, &Faction, &Position)>,
) {
    for (morale, mut order, _faction, _pos) in query.iter_mut() {
        // Broken morale forces retreat
        if morale.is_broken() {
            if !matches!(*order, Order::Retreat) {
                *order = Order::Retreat;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morale_decreases_when_suppressed() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(1.0));

        let entity = world
            .spawn((
                Morale::new(1.0),
                Suppression { value: 0.6 }, // Suppressed
                Health::default(),
                Position::new(0.0, 0.0),
                Faction::Blue,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(morale_system);
        schedule.run(&mut world);

        let morale = world.get::<Morale>(entity).unwrap();
        assert!(morale.value < 1.0);
    }

    #[test]
    fn test_morale_recovers_when_not_suppressed() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(1.0));

        let entity = world
            .spawn((
                Morale::new(0.7),
                Suppression::default(), // Not suppressed
                Health::default(),
                Position::new(0.0, 0.0),
                Faction::Blue,
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(morale_system);
        schedule.run(&mut world);

        let morale = world.get::<Morale>(entity).unwrap();
        assert!(morale.value > 0.7);
    }

    #[test]
    fn test_rout_when_broken() {
        let mut world = World::new();

        let entity = world
            .spawn((
                Morale::new(0.1), // Broken
                Order::Hold,
                Faction::Blue,
                Position::new(0.0, 0.0),
            ))
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(rout_system);
        schedule.run(&mut world);

        let order = world.get::<Order>(entity).unwrap();
        assert!(matches!(*order, Order::Retreat));
    }
}
