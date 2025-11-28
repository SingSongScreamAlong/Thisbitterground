//! Terrain damage system - handles crater creation and terrain deformation events.

use crate::components::*;
use bevy_ecs::prelude::*;

/// System that clears terrain damage events after they've been processed.
/// Events are consumed by the snapshot system and then removed.
pub fn clear_terrain_damage_system(mut commands: Commands, query: Query<Entity, With<TerrainDamageEvent>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

/// Helper function to spawn a terrain damage event (e.g., from artillery).
pub fn spawn_terrain_damage(commands: &mut Commands, x: f32, y: f32, radius: f32, depth: f32) {
    commands.spawn(TerrainDamageEvent { x, y, radius, depth });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_damage_cleared() {
        let mut world = World::new();

        // Spawn a terrain damage event
        world.spawn(TerrainDamageEvent {
            x: 10.0,
            y: 20.0,
            radius: 5.0,
            depth: 1.0,
        });

        // Verify it exists
        let mut query = world.query::<&TerrainDamageEvent>();
        assert_eq!(query.iter(&world).count(), 1);

        // Run clear system
        let mut schedule = Schedule::default();
        schedule.add_systems(clear_terrain_damage_system);
        schedule.run(&mut world);

        // Verify it's gone
        assert_eq!(query.iter(&world).count(), 0);
    }
}
