//! Suppression system - handles suppression decay over time.

use crate::components::*;
use crate::systems::movement::DeltaTime;
use bevy_ecs::prelude::*;

/// Rate at which suppression decays per second.
const SUPPRESSION_DECAY_RATE: f32 = 0.15;

/// System that decays suppression over time.
pub fn suppression_decay_system(dt: Res<DeltaTime>, mut query: Query<&mut Suppression>) {
    let delta = dt.0;
    for mut suppression in query.iter_mut() {
        suppression.decay(SUPPRESSION_DECAY_RATE, delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppression_decays() {
        let mut world = World::new();
        world.insert_resource(DeltaTime(1.0));

        let entity = world
            .spawn(Suppression { value: 1.0 })
            .id();

        let mut schedule = Schedule::default();
        schedule.add_systems(suppression_decay_system);
        schedule.run(&mut world);

        let sup = world.get::<Suppression>(entity).unwrap();
        assert!(sup.value < 1.0);
        assert!((sup.value - (1.0 - SUPPRESSION_DECAY_RATE)).abs() < 0.001);
    }
}
