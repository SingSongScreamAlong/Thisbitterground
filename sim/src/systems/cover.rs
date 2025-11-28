//! Cover system - handles terrain cover and defensive bonuses.

use crate::components::*;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Cover type enumeration.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CoverType {
    #[default]
    None,
    Light,   // Craters, debris - 20% damage reduction
    Medium,  // Trenches, walls - 40% damage reduction
    Heavy,   // Bunkers, buildings - 60% damage reduction
}

impl CoverType {
    /// Get the damage reduction multiplier (0.0 = full damage, 1.0 = no damage).
    pub fn damage_reduction(&self) -> f32 {
        match self {
            CoverType::None => 0.0,
            CoverType::Light => 0.2,
            CoverType::Medium => 0.4,
            CoverType::Heavy => 0.6,
        }
    }

    /// Get the suppression reduction multiplier.
    pub fn suppression_reduction(&self) -> f32 {
        match self {
            CoverType::None => 0.0,
            CoverType::Light => 0.1,
            CoverType::Medium => 0.3,
            CoverType::Heavy => 0.5,
        }
    }

    /// Get the accuracy penalty for firing from this cover (some cover restricts firing).
    pub fn accuracy_penalty(&self) -> f32 {
        match self {
            CoverType::None => 0.0,
            CoverType::Light => 0.0,
            CoverType::Medium => 0.1,
            CoverType::Heavy => 0.2,
        }
    }
}

/// Component indicating a squad is in cover.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct InCover {
    pub cover_type: CoverType,
}

/// Resource containing cover zones on the battlefield.
#[derive(Resource, Default)]
pub struct CoverZones {
    /// List of cover zones: (x, y, radius, cover_type)
    pub zones: Vec<(f32, f32, f32, CoverType)>,
}

impl CoverZones {
    /// Add a cover zone.
    pub fn add_zone(&mut self, x: f32, y: f32, radius: f32, cover_type: CoverType) {
        self.zones.push((x, y, radius, cover_type));
    }

    /// Get the best cover at a position.
    pub fn get_cover_at(&self, x: f32, y: f32) -> CoverType {
        let mut best = CoverType::None;
        for (zx, zy, radius, cover_type) in &self.zones {
            let dx = x - zx;
            let dy = y - zy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist <= *radius {
                // Take the better cover
                if cover_type.damage_reduction() > best.damage_reduction() {
                    best = *cover_type;
                }
            }
        }
        best
    }
}

/// System that updates squad cover status based on position.
pub fn cover_detection_system(
    cover_zones: Option<Res<CoverZones>>,
    mut query: Query<(&Position, &mut InCover)>,
) {
    let zones = match cover_zones {
        Some(z) => z,
        None => return,
    };

    for (pos, mut in_cover) in query.iter_mut() {
        in_cover.cover_type = zones.get_cover_at(pos.x, pos.y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cover_zones() {
        let mut zones = CoverZones::default();
        zones.add_zone(0.0, 0.0, 10.0, CoverType::Medium);
        zones.add_zone(20.0, 0.0, 5.0, CoverType::Heavy);

        assert_eq!(zones.get_cover_at(0.0, 0.0), CoverType::Medium);
        assert_eq!(zones.get_cover_at(5.0, 0.0), CoverType::Medium);
        assert_eq!(zones.get_cover_at(20.0, 0.0), CoverType::Heavy);
        assert_eq!(zones.get_cover_at(100.0, 100.0), CoverType::None);
    }

    #[test]
    fn test_cover_damage_reduction() {
        assert_eq!(CoverType::None.damage_reduction(), 0.0);
        assert_eq!(CoverType::Light.damage_reduction(), 0.2);
        assert_eq!(CoverType::Medium.damage_reduction(), 0.4);
        assert_eq!(CoverType::Heavy.damage_reduction(), 0.6);
    }
}
