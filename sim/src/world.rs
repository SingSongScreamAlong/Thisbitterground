//! Simulation world container and snapshot types.
//!
//! The `Snapshot` struct provides a serializable view of the simulation state
//! that can be sent to Godot for visualization.

use crate::components::*;
use crate::terrain::Crater;
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Snapshot of a single squad's state for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SquadSnapshot {
    pub id: u32,
    pub faction: String,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub health: f32,
    pub health_max: f32,
    pub size: u32,
    pub morale: f32,
    pub suppression: f32,
    pub order: String,
}

/// Snapshot of a terrain damage event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainDamageSnapshot {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub depth: f32,
}

/// Snapshot of a destructible object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DestructibleSnapshot {
    pub id: u32,
    pub x: f32,
    pub y: f32,
    pub dtype: String,
    pub state: String,
    pub health: f32,
    pub health_max: f32,
}

/// Complete simulation state snapshot for Godot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Snapshot {
    /// Current simulation tick.
    pub tick: u64,
    /// Elapsed simulation time in seconds.
    pub time: f32,
    /// All squad states.
    pub squads: Vec<SquadSnapshot>,
    /// All destructible objects.
    pub destructibles: Vec<DestructibleSnapshot>,
    /// Terrain damage events this tick.
    pub terrain_damage: Vec<TerrainDamageSnapshot>,
    /// New craters since last snapshot.
    pub new_craters: Vec<Crater>,
    /// Whether terrain has been modified this tick.
    pub terrain_dirty: bool,
}

impl Snapshot {
    /// Create a snapshot from the ECS world.
    pub fn from_world(world: &mut World, tick: u64, time: f32) -> Self {
        let mut squads = Vec::new();

        // Query all squads
        let mut query = world.query::<(
            &SquadId,
            &Faction,
            &Position,
            &Velocity,
            &Health,
            &SquadStats,
            &Morale,
            &Suppression,
            &Order,
        )>();

        for (squad_id, faction, pos, vel, health, stats, morale, suppression, order) in
            query.iter(world)
        {
            let faction_str = match faction {
                Faction::Blue => "Blue",
                Faction::Red => "Red",
            };

            let order_str = match order {
                Order::Hold => "Hold".to_string(),
                Order::MoveTo { x, y } => format!("MoveTo({:.1},{:.1})", x, y),
                Order::AttackMove { x, y } => format!("AttackMove({:.1},{:.1})", x, y),
                Order::Retreat => "Retreat".to_string(),
            };

            squads.push(SquadSnapshot {
                id: squad_id.0,
                faction: faction_str.to_string(),
                x: pos.x,
                y: pos.y,
                vx: vel.vx,
                vy: vel.vy,
                health: health.current,
                health_max: health.max,
                size: stats.size,
                morale: morale.value,
                suppression: suppression.value,
                order: order_str,
            });
        }

        // Query terrain damage events
        let mut terrain_damage = Vec::new();
        let mut damage_query = world.query::<&TerrainDamageEvent>();
        for event in damage_query.iter(world) {
            terrain_damage.push(TerrainDamageSnapshot {
                x: event.x,
                y: event.y,
                radius: event.radius,
                depth: event.depth,
            });
        }

        // Query destructibles
        let mut destructibles = Vec::new();
        let mut dest_query = world.query::<(
            &DestructibleId,
            &Position,
            &DestructibleType,
            &DestructibleState,
            &DestructibleHealth,
        )>();
        for (id, pos, dtype, state, health) in dest_query.iter(world) {
            let dtype_str = match dtype {
                DestructibleType::Tree => "Tree",
                DestructibleType::Building => "Building",
                DestructibleType::Wall => "Wall",
                DestructibleType::Vehicle => "Vehicle",
            };
            let state_str = match state {
                DestructibleState::Intact => "Intact",
                DestructibleState::Damaged => "Damaged",
                DestructibleState::Destroyed => "Destroyed",
            };
            destructibles.push(DestructibleSnapshot {
                id: id.0,
                x: pos.x,
                y: pos.y,
                dtype: dtype_str.to_string(),
                state: state_str.to_string(),
                health: health.current,
                health_max: health.max,
            });
        }

        Self {
            tick,
            time,
            squads,
            destructibles,
            terrain_damage,
            new_craters: Vec::new(),
            terrain_dirty: false,
        }
    }

    /// Serialize snapshot to JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Serialize snapshot to pretty JSON string.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
