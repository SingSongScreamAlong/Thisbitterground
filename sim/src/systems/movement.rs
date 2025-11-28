//! Movement system - applies velocity to position and handles orders.

use crate::components::*;
use crate::terrain::TerrainResource;
use bevy_ecs::prelude::*;

/// Resource containing the delta time for the current tick.
#[derive(Resource, Default)]
pub struct DeltaTime(pub f32);

/// System that applies velocity to position.
/// Takes terrain into account for movement speed.
pub fn movement_system(
    dt: Res<DeltaTime>,
    terrain: Option<Res<TerrainResource>>,
    mut query: Query<(&mut Position, &Velocity, &Suppression, &Morale)>,
) {
    let delta = dt.0;
    for (mut pos, vel, suppression, morale) in query.iter_mut() {
        // Don't move if pinned or broken
        if suppression.is_pinned() || morale.is_broken() {
            continue;
        }

        // Reduce speed if suppressed or shaken
        let mut speed_mult = if suppression.is_suppressed() {
            0.3
        } else if morale.is_shaken() {
            0.6
        } else {
            1.0
        };

        // Apply terrain movement modifier
        if let Some(ref terrain_res) = terrain {
            let terrain_mult = terrain_res.get_movement_multiplier(pos.x, pos.y);
            speed_mult *= terrain_mult;
        }

        pos.x += vel.vx * delta * speed_mult;
        pos.y += vel.vy * delta * speed_mult;
    }
}

/// System that updates velocity based on orders.
pub fn order_system(
    mut query: Query<(&mut Velocity, &Position, &Order, &SquadStats, &Suppression, &Morale)>,
) {
    for (mut vel, pos, order, stats, suppression, morale) in query.iter_mut() {
        // Can't execute orders if pinned or broken
        if suppression.is_pinned() || morale.is_broken() {
            vel.vx = 0.0;
            vel.vy = 0.0;
            continue;
        }

        match order {
            Order::Hold => {
                vel.vx = 0.0;
                vel.vy = 0.0;
            }
            Order::MoveTo { x, y } | Order::AttackMove { x, y } => {
                let dx = x - pos.x;
                let dy = y - pos.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 1.0 {
                    // Arrived at destination
                    vel.vx = 0.0;
                    vel.vy = 0.0;
                } else {
                    // Move toward target
                    let speed = if matches!(order, Order::AttackMove { .. }) {
                        stats.speed * 0.6 // Attack-move is slower
                    } else {
                        stats.speed
                    };
                    vel.vx = (dx / dist) * speed;
                    vel.vy = (dy / dist) * speed;
                }
            }
            Order::Retreat => {
                // For now, just stop. Later: move away from enemies.
                vel.vx = 0.0;
                vel.vy = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_applies_velocity() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(1.0));

        world.spawn((
            Position::new(0.0, 0.0),
            Velocity::new(5.0, 3.0),
            Suppression::default(),
            Morale::default(),
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let mut query = world.query::<&Position>();
        let pos = query.single(&world);
        assert!((pos.x - 5.0).abs() < 0.001);
        assert!((pos.y - 3.0).abs() < 0.001);
    }
}
