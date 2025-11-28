//! Destruction system - handles damage and state transitions for destructibles.

use crate::components::*;
use bevy_ecs::prelude::*;

/// System that updates destructible states based on health.
pub fn destruction_state_system(
    mut query: Query<(&DestructibleHealth, &mut DestructibleState), Changed<DestructibleHealth>>,
) {
    for (health, mut state) in query.iter_mut() {
        let new_state = if health.is_destroyed() {
            DestructibleState::Destroyed
        } else if health.is_damaged() {
            DestructibleState::Damaged
        } else {
            DestructibleState::Intact
        };

        if *state != new_state {
            *state = new_state;
        }
    }
}

/// System that applies terrain damage events to nearby destructibles.
pub fn terrain_damage_to_destructibles_system(
    damage_events: Query<&TerrainDamageEvent>,
    mut destructibles: Query<(&Position, &mut DestructibleHealth)>,
) {
    for event in damage_events.iter() {
        let event_pos = Position::new(event.x, event.y);
        let damage_radius = event.radius * 1.5; // Slightly larger than crater
        let base_damage = event.depth * 20.0; // Scale damage by crater depth

        for (pos, mut health) in destructibles.iter_mut() {
            let dist = pos.distance_to(&event_pos);
            if dist <= damage_radius {
                // Damage falloff based on distance
                let falloff = 1.0 - (dist / damage_radius);
                let damage = base_damage * falloff * falloff;
                health.damage(damage);
            }
        }
    }
}

/// Collected destruction events for this tick.
#[derive(Debug, Clone, Default)]
pub struct DestructionEvents {
    pub destroyed: Vec<(u32, f32, f32, DestructibleType)>, // (id, x, y, type)
    pub damaged: Vec<(u32, f32, f32, DestructibleType)>,
}

/// Resource to track destruction events for Godot.
#[derive(Resource, Default)]
pub struct DestructionEventBuffer {
    pub events: DestructionEvents,
}

impl DestructionEventBuffer {
    pub fn clear(&mut self) {
        self.events.destroyed.clear();
        self.events.damaged.clear();
    }
}

/// System that collects destruction events for visualization.
pub fn collect_destruction_events_system(
    mut buffer: ResMut<DestructionEventBuffer>,
    query: Query<
        (&DestructibleId, &Position, &DestructibleState, &DestructibleType),
        Changed<DestructibleState>,
    >,
) {
    for (id, pos, state, dtype) in query.iter() {
        match state {
            DestructibleState::Destroyed => {
                buffer.events.destroyed.push((id.0, pos.x, pos.y, *dtype));
            }
            DestructibleState::Damaged => {
                buffer.events.damaged.push((id.0, pos.x, pos.y, *dtype));
            }
            DestructibleState::Intact => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_destruction_state_transitions() {
        let mut world = World::new();

        // Spawn a tree with full health
        let tree = world.spawn((
            DestructibleId(1),
            Position::new(0.0, 0.0),
            DestructibleHealth::new(30.0),
            DestructibleState::Intact,
            DestructibleType::Tree,
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(destruction_state_system);

        // Damage the tree below threshold
        world.get_mut::<DestructibleHealth>(tree).unwrap().damage(20.0);
        schedule.run(&mut world);

        let state = world.get::<DestructibleState>(tree).unwrap();
        assert_eq!(*state, DestructibleState::Damaged);

        // Destroy the tree
        world.get_mut::<DestructibleHealth>(tree).unwrap().damage(20.0);
        schedule.run(&mut world);

        let state = world.get::<DestructibleState>(tree).unwrap();
        assert_eq!(*state, DestructibleState::Destroyed);
    }

    #[test]
    fn test_terrain_damage_affects_destructibles() {
        let mut world = World::new();

        // Spawn a tree
        world.spawn((
            DestructibleId(1),
            Position::new(5.0, 0.0),
            DestructibleHealth::new(30.0),
            DestructibleState::Intact,
        ));

        // Spawn a terrain damage event nearby
        world.spawn(TerrainDamageEvent {
            x: 0.0,
            y: 0.0,
            radius: 10.0,
            depth: 2.0,
        });

        let mut schedule = Schedule::default();
        schedule.add_systems(terrain_damage_to_destructibles_system);
        schedule.run(&mut world);

        // Tree should have taken damage
        let mut query = world.query::<&DestructibleHealth>();
        let health = query.single(&world);
        assert!(health.current < 30.0, "Tree should have taken damage from nearby crater");
    }
}
