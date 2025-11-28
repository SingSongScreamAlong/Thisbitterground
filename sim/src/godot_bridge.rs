//! Godot Integration Bridge
//!
//! This module provides the interface between the Rust ECS simulation and Godot/GDExtension.
//! It handles conversion of simulation state into FFI-friendly formats for efficient
//! cross-language communication.
//!
//! # Stable FFI Contract
//!
//! This module defines a **stable binary format** for transferring simulation state to Godot.
//! The format is designed for:
//! - **Efficiency**: Contiguous f32 array, no allocations on the Godot side
//! - **Simplicity**: Fixed stride, predictable layout
//! - **Stability**: Field order and count are versioned and documented
//!
//! # Buffer Layout (Version 1.0)
//!
//! The flat buffer is a `Vec<f32>` with the following structure:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ HEADER (1 element)                                              │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ [0] squad_count (as f32)                                        │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ SQUAD DATA (squad_count × SQUAD_STRIDE elements)                │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ For each squad i (offset = 1 + i * SQUAD_STRIDE):               │
//! │   [+0]  id          - Squad ID (u32 as f32)                     │
//! │   [+1]  x           - X position (world units)                  │
//! │   [+2]  y           - Y position (world units)                  │
//! │   [+3]  vx          - X velocity (units/sec)                    │
//! │   [+4]  vy          - Y velocity (units/sec)                    │
//! │   [+5]  faction_id  - Faction (0.0=Blue, 1.0=Red)               │
//! │   [+6]  size        - Squad size (soldier count as f32)         │
//! │   [+7]  health      - Current health points                     │
//! │   [+8]  health_max  - Maximum health points                     │
//! │   [+9]  morale      - Morale (0.0-1.0)                          │
//! │   [+10] suppression - Suppression level (0.0-1.0)               │
//! │   [+11] is_alive    - Alive flag (1.0=alive, 0.0=dead)          │
//! │   [+12] is_routing  - Routing flag (1.0=routing, 0.0=not)       │
//! │   [+13] order_type  - Order type (see ORDER_* constants)        │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Constants
//!
//! - `SQUAD_STRIDE = 14` - Number of f32 values per squad
//! - `HEADER_SIZE = 1` - Number of f32 values in header
//!
//! # Faction ID Mapping
//!
//! | Faction | ID |
//! |---------|-----|
//! | Blue    | 0.0 |
//! | Red     | 1.0 |
//!
//! # Order Type Mapping
//!
//! | Order       | ID  |
//! |-------------|-----|
//! | Hold        | 0.0 |
//! | MoveTo      | 1.0 |
//! | AttackMove  | 2.0 |
//! | Retreat     | 3.0 |
//!
//! # Usage from Godot (GDScript)
//!
//! ```gdscript
//! const SQUAD_STRIDE = 14
//! const HEADER_SIZE = 1
//!
//! func parse_snapshot(buffer: PackedFloat32Array):
//!     var squad_count = int(buffer[0])
//!     for i in range(squad_count):
//!         var offset = HEADER_SIZE + i * SQUAD_STRIDE
//!         var squad_id = int(buffer[offset + 0])
//!         var x = buffer[offset + 1]
//!         var y = buffer[offset + 2]
//!         var faction = int(buffer[offset + 5])  # 0=Blue, 1=Red
//!         var health = buffer[offset + 7]
//!         var is_alive = buffer[offset + 11] > 0.5
//!         # ... update visual representation
//! ```
//!
//! # Determinism
//!
//! The buffer is deterministic: given the same `Snapshot`, the output is identical.
//! Squads are serialized in their existing order (no sorting applied).

use crate::world::Snapshot;

// ============================================================================
// CONSTANTS - STABLE FFI CONTRACT
// ============================================================================

/// Number of f32 values per squad in the flat buffer.
/// 
/// **This is part of the stable FFI contract. Do not change without versioning.**
/// 
/// Fields (in order):
/// 0. id, 1. x, 2. y, 3. vx, 4. vy, 5. faction_id, 6. size,
/// 7. health, 8. health_max, 9. morale, 10. suppression,
/// 11. is_alive, 12. is_routing, 13. order_type
pub const SQUAD_STRIDE: usize = 14;

/// Number of f32 values in the buffer header.
/// Currently just squad_count.
pub const HEADER_SIZE: usize = 1;

// Order type constants for FFI
/// Order type: Hold position
pub const ORDER_HOLD: f32 = 0.0;
/// Order type: Move to location
pub const ORDER_MOVE_TO: f32 = 1.0;
/// Order type: Attack-move to location
pub const ORDER_ATTACK_MOVE: f32 = 2.0;
/// Order type: Retreat
pub const ORDER_RETREAT: f32 = 3.0;

// Faction ID constants for FFI
/// Faction ID: Blue team
pub const FACTION_BLUE: f32 = 0.0;
/// Faction ID: Red team
pub const FACTION_RED: f32 = 1.0;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert a faction string to its numeric ID for FFI.
/// 
/// # Mapping
/// - "Blue" → 0.0
/// - "Red" → 1.0
/// - Unknown → 0.0 (defaults to Blue)
#[inline]
pub fn faction_to_id(faction: &str) -> f32 {
    match faction {
        "Blue" => FACTION_BLUE,
        "Red" => FACTION_RED,
        _ => FACTION_BLUE, // Default to Blue for unknown
    }
}

/// Convert an order string to its numeric ID for FFI.
/// 
/// # Mapping
/// - "Hold" → 0.0
/// - "MoveTo(...)" → 1.0
/// - "AttackMove(...)" → 2.0
/// - "Retreat" → 3.0
/// - Unknown → 0.0 (defaults to Hold)
#[inline]
pub fn order_to_id(order: &str) -> f32 {
    if order == "Hold" {
        ORDER_HOLD
    } else if order.starts_with("MoveTo") {
        ORDER_MOVE_TO
    } else if order.starts_with("AttackMove") {
        ORDER_ATTACK_MOVE
    } else if order == "Retreat" {
        ORDER_RETREAT
    } else {
        ORDER_HOLD // Default to Hold for unknown
    }
}

/// Check if a squad is routing based on its order.
#[inline]
fn is_routing(order: &str) -> f32 {
    if order == "Retreat" { 1.0 } else { 0.0 }
}

/// Check if a squad is alive based on its health.
#[inline]
fn is_alive(health: f32) -> f32 {
    if health > 0.0 { 1.0 } else { 0.0 }
}

// ============================================================================
// MAIN SERIALIZATION FUNCTION
// ============================================================================

/// Convert a simulation snapshot to a flat buffer for FFI transfer to Godot.
///
/// # Buffer Format
///
/// See module-level documentation for the complete buffer layout.
///
/// # Layout Summary
///
/// - `buffer[0]` = squad_count (as f32)
/// - For each squad `i` at offset `1 + i * SQUAD_STRIDE`:
///   - id, x, y, vx, vy, faction_id, size, health, health_max,
///     morale, suppression, is_alive, is_routing, order_type
///
/// # Determinism
///
/// This function is deterministic: the same `Snapshot` always produces
/// the same output buffer. Squads are serialized in their existing order.
///
/// # Example
///
/// ```rust
/// use tbg_sim::api::SimWorld;
/// use tbg_sim::godot_bridge::{snapshot_to_flatbuffer, SQUAD_STRIDE, HEADER_SIZE};
///
/// let mut sim = SimWorld::new();
/// let snapshot = sim.snapshot();
/// let buffer = snapshot_to_flatbuffer(&snapshot);
///
/// // Buffer length is header + (squad_count * stride)
/// let squad_count = buffer[0] as usize;
/// assert_eq!(buffer.len(), HEADER_SIZE + squad_count * SQUAD_STRIDE);
/// ```
pub fn snapshot_to_flatbuffer(snapshot: &Snapshot) -> Vec<f32> {
    let squad_count = snapshot.squads.len();
    let buffer_size = HEADER_SIZE + squad_count * SQUAD_STRIDE;
    
    // Pre-allocate exact capacity
    let mut buffer = Vec::with_capacity(buffer_size);
    
    // Header: squad count
    buffer.push(squad_count as f32);
    
    // Squad data: fixed stride per squad
    for squad in &snapshot.squads {
        // [+0] id
        buffer.push(squad.id as f32);
        // [+1] x
        buffer.push(squad.x);
        // [+2] y
        buffer.push(squad.y);
        // [+3] vx
        buffer.push(squad.vx);
        // [+4] vy
        buffer.push(squad.vy);
        // [+5] faction_id
        buffer.push(faction_to_id(&squad.faction));
        // [+6] size
        buffer.push(squad.size as f32);
        // [+7] health
        buffer.push(squad.health);
        // [+8] health_max
        buffer.push(squad.health_max);
        // [+9] morale
        buffer.push(squad.morale);
        // [+10] suppression
        buffer.push(squad.suppression);
        // [+11] is_alive
        buffer.push(is_alive(squad.health));
        // [+12] is_routing
        buffer.push(is_routing(&squad.order));
        // [+13] order_type
        buffer.push(order_to_id(&squad.order));
    }
    
    debug_assert_eq!(buffer.len(), buffer_size, "Buffer size mismatch");
    buffer
}

/// Calculate the required buffer size for a given squad count.
/// 
/// # Formula
/// `HEADER_SIZE + squad_count * SQUAD_STRIDE`
#[inline]
pub fn calculate_buffer_size(squad_count: usize) -> usize {
    HEADER_SIZE + squad_count * SQUAD_STRIDE
}

/// Parse the squad count from a flat buffer.
///
/// Returns `None` if the buffer is empty.
#[inline]
pub fn parse_squad_count(buffer: &[f32]) -> Option<usize> {
    if buffer.is_empty() {
        return None;
    }
    Some(buffer[0] as usize)
}

/// Get the buffer offset for a specific squad index.
/// 
/// # Example
/// ```rust
/// use tbg_sim::godot_bridge::squad_offset;
/// 
/// let offset = squad_offset(0); // First squad at offset 1
/// let offset = squad_offset(5); // Sixth squad
/// ```
#[inline]
pub const fn squad_offset(squad_index: usize) -> usize {
    HEADER_SIZE + squad_index * SQUAD_STRIDE
}

// ============================================================================
// FIELD OFFSET CONSTANTS (for Godot-side parsing)
// ============================================================================

/// Offset within squad data for: ID
pub const FIELD_ID: usize = 0;
/// Offset within squad data for: X position
pub const FIELD_X: usize = 1;
/// Offset within squad data for: Y position
pub const FIELD_Y: usize = 2;
/// Offset within squad data for: X velocity
pub const FIELD_VX: usize = 3;
/// Offset within squad data for: Y velocity
pub const FIELD_VY: usize = 4;
/// Offset within squad data for: Faction ID
pub const FIELD_FACTION: usize = 5;
/// Offset within squad data for: Squad size
pub const FIELD_SIZE: usize = 6;
/// Offset within squad data for: Current health
pub const FIELD_HEALTH: usize = 7;
/// Offset within squad data for: Max health
pub const FIELD_HEALTH_MAX: usize = 8;
/// Offset within squad data for: Morale
pub const FIELD_MORALE: usize = 9;
/// Offset within squad data for: Suppression
pub const FIELD_SUPPRESSION: usize = 10;
/// Offset within squad data for: Is alive flag
pub const FIELD_IS_ALIVE: usize = 11;
/// Offset within squad data for: Is routing flag
pub const FIELD_IS_ROUTING: usize = 12;
/// Offset within squad data for: Order type
pub const FIELD_ORDER_TYPE: usize = 13;

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SimWorld;
    use crate::components::Faction;
    use crate::systems::SimConfig;

    #[test]
    fn test_snapshot_to_flatbuffer_empty() {
        let mut sim = SimWorld::new();
        let snapshot = sim.snapshot();
        let buffer = snapshot_to_flatbuffer(&snapshot);
        
        // Empty world: just header with count=0
        assert_eq!(buffer.len(), HEADER_SIZE);
        assert_eq!(buffer[0], 0.0);
    }

    #[test]
    fn test_snapshot_to_flatbuffer_with_squads() {
        let config = SimConfig::default();
        let mut sim = SimWorld::with_config(config);
        
        // Spawn some squads using existing helpers
        sim.spawn_ai_squad(1, Faction::Blue, 10.0, 20.0);
        sim.spawn_ai_squad(2, Faction::Red, 50.0, 60.0);
        sim.spawn_ai_squad(3, Faction::Blue, -30.0, -40.0);
        
        let snapshot = sim.snapshot();
        let buffer = snapshot_to_flatbuffer(&snapshot);
        
        // Verify buffer length
        let squad_count = 3;
        let expected_len = HEADER_SIZE + squad_count * SQUAD_STRIDE;
        assert_eq!(buffer.len(), expected_len, "Buffer length mismatch");
        
        // Verify header
        assert_eq!(buffer[0], squad_count as f32, "Squad count mismatch");
        
        // Verify first squad data at correct offset
        let offset = squad_offset(0);
        assert_eq!(buffer[offset + FIELD_ID], 1.0, "Squad 1 ID");
        assert_eq!(buffer[offset + FIELD_X], 10.0, "Squad 1 X");
        assert_eq!(buffer[offset + FIELD_Y], 20.0, "Squad 1 Y");
        assert_eq!(buffer[offset + FIELD_FACTION], FACTION_BLUE, "Squad 1 faction");
        assert_eq!(buffer[offset + FIELD_IS_ALIVE], 1.0, "Squad 1 alive");
        
        // Verify second squad
        let offset = squad_offset(1);
        assert_eq!(buffer[offset + FIELD_ID], 2.0, "Squad 2 ID");
        assert_eq!(buffer[offset + FIELD_X], 50.0, "Squad 2 X");
        assert_eq!(buffer[offset + FIELD_Y], 60.0, "Squad 2 Y");
        assert_eq!(buffer[offset + FIELD_FACTION], FACTION_RED, "Squad 2 faction");
        
        // Verify third squad
        let offset = squad_offset(2);
        assert_eq!(buffer[offset + FIELD_ID], 3.0, "Squad 3 ID");
        assert_eq!(buffer[offset + FIELD_X], -30.0, "Squad 3 X");
        assert_eq!(buffer[offset + FIELD_Y], -40.0, "Squad 3 Y");
        assert_eq!(buffer[offset + FIELD_FACTION], FACTION_BLUE, "Squad 3 faction");
    }

    #[test]
    fn test_snapshot_to_flatbuffer_determinism() {
        // Create two identical SimWorlds
        let config = SimConfig::default();
        
        let mut sim1 = SimWorld::with_config(config.clone());
        sim1.spawn_ai_squad(100, Faction::Blue, 0.0, 0.0);
        sim1.spawn_ai_squad(200, Faction::Red, 100.0, 100.0);
        
        let mut sim2 = SimWorld::with_config(config);
        sim2.spawn_ai_squad(100, Faction::Blue, 0.0, 0.0);
        sim2.spawn_ai_squad(200, Faction::Red, 100.0, 100.0);
        
        // Take snapshots
        let snapshot1 = sim1.snapshot();
        let snapshot2 = sim2.snapshot();
        
        // Serialize both
        let buffer1 = snapshot_to_flatbuffer(&snapshot1);
        let buffer2 = snapshot_to_flatbuffer(&snapshot2);
        
        // Buffers must be identical
        assert_eq!(buffer1.len(), buffer2.len(), "Buffer lengths differ");
        assert_eq!(buffer1, buffer2, "Buffers are not identical - determinism violated");
    }

    #[test]
    fn test_snapshot_to_flatbuffer_after_simulation() {
        let config = SimConfig::default();
        let mut sim = SimWorld::with_config(config);
        
        // Spawn opposing squads
        sim.spawn_ai_squad(1, Faction::Blue, -50.0, 0.0);
        sim.spawn_ai_squad(2, Faction::Red, 50.0, 0.0);
        
        // Run simulation for a few ticks
        for _ in 0..10 {
            sim.step(0.033); // ~30 Hz
        }
        
        let snapshot = sim.snapshot();
        let buffer = snapshot_to_flatbuffer(&snapshot);
        
        // Verify structure is still valid
        let squad_count = buffer[0] as usize;
        assert_eq!(buffer.len(), HEADER_SIZE + squad_count * SQUAD_STRIDE);
        
        // Squads should still exist (may have taken damage)
        assert!(squad_count >= 1, "At least one squad should remain");
        
        // Verify all squads have valid data
        for i in 0..squad_count {
            let offset = squad_offset(i);
            let health = buffer[offset + FIELD_HEALTH];
            let health_max = buffer[offset + FIELD_HEALTH_MAX];
            let is_alive = buffer[offset + FIELD_IS_ALIVE];
            
            // Health should be <= max
            assert!(health <= health_max, "Health exceeds max for squad {}", i);
            // is_alive should match health > 0
            let expected_alive = if health > 0.0 { 1.0 } else { 0.0 };
            assert_eq!(is_alive, expected_alive, "is_alive mismatch for squad {}", i);
        }
    }

    #[test]
    fn test_calculate_buffer_size() {
        assert_eq!(calculate_buffer_size(0), HEADER_SIZE);
        assert_eq!(calculate_buffer_size(1), HEADER_SIZE + SQUAD_STRIDE);
        assert_eq!(calculate_buffer_size(100), HEADER_SIZE + 100 * SQUAD_STRIDE);
    }

    #[test]
    fn test_parse_squad_count() {
        let buffer: Vec<f32> = vec![];
        assert_eq!(parse_squad_count(&buffer), None);
        
        let buffer = vec![5.0];
        assert_eq!(parse_squad_count(&buffer), Some(5));
        
        let buffer = vec![0.0];
        assert_eq!(parse_squad_count(&buffer), Some(0));
    }

    #[test]
    fn test_squad_offset() {
        assert_eq!(squad_offset(0), HEADER_SIZE);
        assert_eq!(squad_offset(1), HEADER_SIZE + SQUAD_STRIDE);
        assert_eq!(squad_offset(10), HEADER_SIZE + 10 * SQUAD_STRIDE);
    }

    #[test]
    fn test_faction_to_id() {
        assert_eq!(faction_to_id("Blue"), FACTION_BLUE);
        assert_eq!(faction_to_id("Red"), FACTION_RED);
        assert_eq!(faction_to_id("Unknown"), FACTION_BLUE); // Default
    }

    #[test]
    fn test_order_to_id() {
        assert_eq!(order_to_id("Hold"), ORDER_HOLD);
        assert_eq!(order_to_id("MoveTo(10.0,20.0)"), ORDER_MOVE_TO);
        assert_eq!(order_to_id("AttackMove(5.0,5.0)"), ORDER_ATTACK_MOVE);
        assert_eq!(order_to_id("Retreat"), ORDER_RETREAT);
        assert_eq!(order_to_id("Unknown"), ORDER_HOLD); // Default
    }

    #[test]
    fn test_field_offsets_are_valid() {
        // Ensure all field offsets are within stride
        assert!(FIELD_ID < SQUAD_STRIDE);
        assert!(FIELD_X < SQUAD_STRIDE);
        assert!(FIELD_Y < SQUAD_STRIDE);
        assert!(FIELD_VX < SQUAD_STRIDE);
        assert!(FIELD_VY < SQUAD_STRIDE);
        assert!(FIELD_FACTION < SQUAD_STRIDE);
        assert!(FIELD_SIZE < SQUAD_STRIDE);
        assert!(FIELD_HEALTH < SQUAD_STRIDE);
        assert!(FIELD_HEALTH_MAX < SQUAD_STRIDE);
        assert!(FIELD_MORALE < SQUAD_STRIDE);
        assert!(FIELD_SUPPRESSION < SQUAD_STRIDE);
        assert!(FIELD_IS_ALIVE < SQUAD_STRIDE);
        assert!(FIELD_IS_ROUTING < SQUAD_STRIDE);
        assert!(FIELD_ORDER_TYPE < SQUAD_STRIDE);
        
        // Ensure stride matches the highest field + 1
        assert_eq!(SQUAD_STRIDE, FIELD_ORDER_TYPE + 1);
    }
}
