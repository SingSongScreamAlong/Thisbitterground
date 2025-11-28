//! ECS Components for This Bitter Ground simulation.
//!
//! Components are pure data containers attached to entities.
//! All game logic lives in systems that query these components.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// SPATIAL COMPONENTS
// ============================================================================

/// 2D position on the battlefield (x = east/west, y = north/south).
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

impl Position {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// 2D velocity vector.
#[derive(Component, Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Velocity {
    pub vx: f32,
    pub vy: f32,
}

impl Velocity {
    pub fn new(vx: f32, vy: f32) -> Self {
        Self { vx, vy }
    }

    pub fn magnitude(&self) -> f32 {
        (self.vx * self.vx + self.vy * self.vy).sqrt()
    }

    pub fn normalized(&self) -> Self {
        let mag = self.magnitude();
        if mag < 0.0001 {
            Self::default()
        } else {
            Self {
                vx: self.vx / mag,
                vy: self.vy / mag,
            }
        }
    }
}

// ============================================================================
// IDENTITY COMPONENTS
// ============================================================================

/// Unique identifier for a squad.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SquadId(pub u32);

/// Faction/side identifier.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Faction {
    Blue,
    Red,
}

// ============================================================================
// COMBAT COMPONENTS
// ============================================================================

/// Health/strength of a unit or squad.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

impl Health {
    pub fn new(max: f32) -> Self {
        Self { current: max, max }
    }

    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 {
            0.0
        } else {
            (self.current / self.max).clamp(0.0, 1.0)
        }
    }

    pub fn is_alive(&self) -> bool {
        self.current > 0.0
    }

    pub fn damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }

    pub fn heal(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
    }
}

impl Default for Health {
    fn default() -> Self {
        Self::new(100.0)
    }
}

/// Squad statistics (aggregated unit data).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SquadStats {
    /// Number of soldiers in the squad.
    pub size: u32,
    /// Base movement speed (units per second).
    pub speed: f32,
    /// Fire range (units).
    pub fire_range: f32,
    /// Base accuracy (0.0 - 1.0).
    pub accuracy: f32,
}

impl Default for SquadStats {
    fn default() -> Self {
        Self {
            size: 12,
            speed: 5.0,
            fire_range: 60.0,
            accuracy: 0.2,
        }
    }
}

// ============================================================================
// MORALE & SUPPRESSION COMPONENTS
// ============================================================================

/// Morale state of a squad (affects behavior and combat effectiveness).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Morale {
    /// Current morale (0.0 = broken, 1.0 = full).
    pub value: f32,
}

impl Default for Morale {
    fn default() -> Self {
        Self { value: 1.0 }
    }
}

impl Morale {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }

    pub fn decrease(&mut self, amount: f32) {
        self.value = (self.value - amount).max(0.0);
    }

    pub fn recover(&mut self, amount: f32) {
        self.value = (self.value + amount).min(1.0);
    }

    pub fn is_broken(&self) -> bool {
        self.value < 0.2
    }

    pub fn is_shaken(&self) -> bool {
        self.value < 0.5
    }
}

/// Suppression level (temporary combat debuff from incoming fire).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Suppression {
    /// Current suppression (0.0 = none, 1.0+ = pinned).
    pub value: f32,
}

impl Default for Suppression {
    fn default() -> Self {
        Self { value: 0.0 }
    }
}

impl Suppression {
    pub fn add(&mut self, amount: f32) {
        self.value += amount;
    }

    pub fn decay(&mut self, rate: f32, dt: f32) {
        self.value = (self.value - rate * dt).max(0.0);
    }

    pub fn is_pinned(&self) -> bool {
        self.value >= 1.0
    }

    pub fn is_suppressed(&self) -> bool {
        self.value >= 0.5
    }
}

// ============================================================================
// AI / ORDER COMPONENTS
// ============================================================================

/// Current order/goal for a squad.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Order {
    /// Stay in place.
    Hold,
    /// Move toward a target position.
    MoveTo { x: f32, y: f32 },
    /// Move toward position while engaging enemies.
    AttackMove { x: f32, y: f32 },
    /// Retreat away from combat.
    Retreat,
}

impl Default for Order {
    fn default() -> Self {
        Self::Hold
    }
}

/// AI behavior state for autonomous decision-making.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BehaviorState {
    /// Idle, waiting for orders or threats.
    Idle,
    /// Advancing toward objective.
    Advancing,
    /// Engaging enemies.
    Engaging,
    /// Taking cover from fire.
    TakingCover,
    /// Flanking enemy position.
    Flanking,
    /// Retreating from combat.
    Retreating,
    /// Regrouping with nearby friendlies.
    Regrouping,
}

impl Default for BehaviorState {
    fn default() -> Self {
        Self::Idle
    }
}

/// Marker for AI-controlled squads.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct AIControlled;

/// Threat awareness tracking for AI decisions.
#[derive(Component, Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreatAwareness {
    /// Position of nearest known enemy.
    pub nearest_enemy: Option<(f32, f32)>,
    /// Distance to nearest enemy.
    pub nearest_enemy_dist: f32,
    /// Number of enemies in engagement range.
    pub enemies_in_range: u32,
    /// Estimated incoming fire direction (normalized).
    pub fire_direction: Option<(f32, f32)>,
    /// Time since last taking fire.
    pub time_since_fire: f32,
    /// Threat level (0.0 = safe, 1.0 = extreme danger).
    pub threat_level: f32,
}

impl ThreatAwareness {
    pub fn is_under_fire(&self) -> bool {
        self.time_since_fire < 2.0
    }

    pub fn has_enemy_contact(&self) -> bool {
        self.nearest_enemy.is_some()
    }

    pub fn clear(&mut self) {
        self.nearest_enemy = None;
        self.nearest_enemy_dist = f32::MAX;
        self.enemies_in_range = 0;
        self.fire_direction = None;
    }
}

/// Flocking behavior weights for swarm movement.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FlockingWeights {
    /// Weight for cohesion (move toward center of nearby friendlies).
    pub cohesion: f32,
    /// Weight for separation (avoid crowding nearby friendlies).
    pub separation: f32,
    /// Weight for alignment (match velocity of nearby friendlies).
    pub alignment: f32,
    /// Weight for goal-seeking (move toward objective).
    pub goal_seeking: f32,
    /// Weight for threat avoidance.
    pub threat_avoidance: f32,
    /// Radius for neighbor detection.
    pub neighbor_radius: f32,
    /// Minimum separation distance.
    pub separation_radius: f32,
}

impl Default for FlockingWeights {
    fn default() -> Self {
        Self {
            cohesion: 0.3,
            separation: 0.5,
            alignment: 0.2,
            goal_seeking: 1.0,
            threat_avoidance: 0.8,
            neighbor_radius: 30.0,
            separation_radius: 8.0,
        }
    }
}

/// Tactical preferences for AI decision-making.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TacticalPreferences {
    /// Aggression level (0.0 = defensive, 1.0 = aggressive).
    pub aggression: f32,
    /// Preference for using cover.
    pub cover_seeking: f32,
    /// Willingness to flank vs direct assault.
    pub flanking_tendency: f32,
    /// How quickly to retreat when threatened.
    pub retreat_threshold: f32,
    /// Coordination with nearby friendlies.
    pub coordination: f32,
}

impl Default for TacticalPreferences {
    fn default() -> Self {
        Self {
            aggression: 0.5,
            cover_seeking: 0.6,
            flanking_tendency: 0.4,
            retreat_threshold: 0.3,
            coordination: 0.7,
        }
    }
}

/// Nearby friendly tracking for coordination.
#[derive(Component, Debug, Clone, Default)]
pub struct NearbyFriendlies {
    /// IDs of friendly squads within coordination range.
    pub squad_ids: Vec<u32>,
    /// Center of mass of nearby friendlies.
    pub center_of_mass: Option<(f32, f32)>,
    /// Average velocity of nearby friendlies.
    pub average_velocity: (f32, f32),
}

// ============================================================================
// PERFORMANCE / LOD COMPONENTS
// ============================================================================

/// Simulation Level-of-Detail for performance optimization.
/// High-LOD entities update every tick, Medium every 2 ticks, Low every 4-8 ticks.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SimLod {
    /// Full simulation fidelity - updates every tick.
    #[default]
    High,
    /// Reduced fidelity - updates every 2 ticks.
    Medium,
    /// Minimal fidelity - updates every 4-8 ticks.
    Low,
}

impl SimLod {
    /// Returns the tick interval for this LOD level.
    pub fn tick_interval(&self) -> u64 {
        match self {
            SimLod::High => 1,
            SimLod::Medium => 2,
            SimLod::Low => 4,
        }
    }

    /// Check if this entity should update on the given tick.
    #[inline]
    pub fn should_update(&self, tick: u64) -> bool {
        tick % self.tick_interval() == 0
    }
}

/// Sector identifier for spatial batching of combat/suppression.
/// Units in the same sector share aggregated combat stats.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct SectorId(pub i32, pub i32);

impl SectorId {
    pub fn from_position(x: f32, y: f32, sector_size: f32) -> Self {
        Self(
            (x / sector_size).floor() as i32,
            (y / sector_size).floor() as i32,
        )
    }
}

/// Activity flags for skipping idle units in heavy systems.
/// Units without any active flags can skip expensive computations.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct ActivityFlags {
    /// Unit has non-zero velocity or pending movement.
    pub is_moving: bool,
    /// Unit is currently firing or has a target.
    pub is_firing: bool,
    /// Unit took damage recently (within last few ticks).
    pub recently_damaged: bool,
    /// Unit is under suppression.
    pub is_suppressed: bool,
    /// Tick when damage was last received.
    pub last_damage_tick: u64,
}

impl ActivityFlags {
    /// Check if unit is considered "active" and needs full processing.
    #[inline]
    pub fn is_active(&self) -> bool {
        self.is_moving || self.is_firing || self.recently_damaged || self.is_suppressed
    }

    /// Check if unit is idle and can skip heavy processing.
    #[inline]
    pub fn is_idle(&self) -> bool {
        !self.is_active()
    }

    /// Update recently_damaged based on current tick.
    pub fn update_damage_status(&mut self, current_tick: u64, damage_memory_ticks: u64) {
        self.recently_damaged = current_tick.saturating_sub(self.last_damage_tick) < damage_memory_ticks;
    }

    /// Mark unit as having taken damage.
    pub fn mark_damaged(&mut self, tick: u64) {
        self.last_damage_tick = tick;
        self.recently_damaged = true;
    }
}

// ============================================================================
// TERRAIN / DESTRUCTIBLE COMPONENTS
// ============================================================================

/// Marker component for terrain damage events (craters, etc.).
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TerrainDamageEvent {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub depth: f32,
}

/// State of a destructible object (tree, building, etc.).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructibleState {
    Intact,
    Damaged,
    Destroyed,
}

impl Default for DestructibleState {
    fn default() -> Self {
        Self::Intact
    }
}

/// Unique identifier for destructible objects.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DestructibleId(pub u32);

impl Default for DestructibleId {
    fn default() -> Self {
        Self(0)
    }
}

/// Health component for destructible objects.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DestructibleHealth {
    pub current: f32,
    pub max: f32,
    /// Threshold below which object becomes "Damaged"
    pub damage_threshold: f32,
}

impl DestructibleHealth {
    pub fn new(max: f32) -> Self {
        Self {
            current: max,
            max,
            damage_threshold: max * 0.5,
        }
    }

    pub fn with_threshold(max: f32, damage_threshold: f32) -> Self {
        Self {
            current: max,
            max,
            damage_threshold,
        }
    }

    pub fn fraction(&self) -> f32 {
        if self.max <= 0.0 { 0.0 } else { (self.current / self.max).clamp(0.0, 1.0) }
    }

    pub fn is_destroyed(&self) -> bool {
        self.current <= 0.0
    }

    pub fn is_damaged(&self) -> bool {
        self.current <= self.damage_threshold && self.current > 0.0
    }

    pub fn damage(&mut self, amount: f32) {
        self.current = (self.current - amount).max(0.0);
    }
}

impl Default for DestructibleHealth {
    fn default() -> Self {
        Self::new(50.0)
    }
}

/// Cover bonus provided by this destructible.
#[derive(Component, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CoverProvider {
    /// Cover value when intact (0.0 - 1.0)
    pub intact_cover: f32,
    /// Cover value when damaged
    pub damaged_cover: f32,
    /// Cover value when destroyed (rubble)
    pub destroyed_cover: f32,
    /// Radius of cover effect
    pub radius: f32,
}

impl Default for CoverProvider {
    fn default() -> Self {
        Self {
            intact_cover: 0.4,
            damaged_cover: 0.2,
            destroyed_cover: 0.1,
            radius: 3.0,
        }
    }
}

impl CoverProvider {
    pub fn tree() -> Self {
        Self {
            intact_cover: 0.3,
            damaged_cover: 0.1,
            destroyed_cover: 0.0,
            radius: 2.0,
        }
    }

    pub fn building() -> Self {
        Self {
            intact_cover: 0.7,
            damaged_cover: 0.5,
            destroyed_cover: 0.3,
            radius: 5.0,
        }
    }

    pub fn get_cover(&self, state: DestructibleState) -> f32 {
        match state {
            DestructibleState::Intact => self.intact_cover,
            DestructibleState::Damaged => self.damaged_cover,
            DestructibleState::Destroyed => self.destroyed_cover,
        }
    }
}

/// Type of destructible object.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestructibleType {
    Tree,
    Building,
    Wall,
    Vehicle,
}

impl Default for DestructibleType {
    fn default() -> Self {
        Self::Tree
    }
}

/// Marker for tree entities.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Tree;

/// Marker for building entities.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct Building;

// ============================================================================
// BUNDLE HELPERS
// ============================================================================

/// Bundle for spawning a complete squad entity.
#[derive(Bundle, Default)]
pub struct SquadBundle {
    pub squad_id: SquadId,
    pub faction: Faction,
    pub position: Position,
    pub velocity: Velocity,
    pub health: Health,
    pub stats: SquadStats,
    pub morale: Morale,
    pub suppression: Suppression,
    pub order: Order,
}

impl Default for Faction {
    fn default() -> Self {
        Self::Blue
    }
}

impl Default for SquadId {
    fn default() -> Self {
        Self(0)
    }
}

/// Bundle for spawning a tree entity.
#[derive(Bundle, Default)]
pub struct TreeBundle {
    pub id: DestructibleId,
    pub position: Position,
    pub health: DestructibleHealth,
    pub state: DestructibleState,
    pub cover: CoverProvider,
    pub dtype: DestructibleType,
    pub marker: Tree,
}

impl TreeBundle {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self {
            id: DestructibleId(id),
            position: Position::new(x, y),
            health: DestructibleHealth::new(30.0),
            state: DestructibleState::Intact,
            cover: CoverProvider::tree(),
            dtype: DestructibleType::Tree,
            marker: Tree,
        }
    }
}

/// Bundle for spawning a building entity.
#[derive(Bundle, Default)]
pub struct BuildingBundle {
    pub id: DestructibleId,
    pub position: Position,
    pub health: DestructibleHealth,
    pub state: DestructibleState,
    pub cover: CoverProvider,
    pub dtype: DestructibleType,
    pub marker: Building,
}

impl BuildingBundle {
    pub fn new(id: u32, x: f32, y: f32) -> Self {
        Self {
            id: DestructibleId(id),
            position: Position::new(x, y),
            health: DestructibleHealth::new(150.0),
            state: DestructibleState::Intact,
            cover: CoverProvider::building(),
            dtype: DestructibleType::Building,
            marker: Building,
        }
    }
}

/// Bundle for AI components to add to a squad.
#[derive(Bundle, Default)]
pub struct AIBundle {
    pub ai_controlled: AIControlled,
    pub behavior_state: BehaviorState,
    pub threat_awareness: ThreatAwareness,
    pub flocking_weights: FlockingWeights,
    pub tactical_prefs: TacticalPreferences,
    pub nearby_friendlies: NearbyFriendlies,
}
