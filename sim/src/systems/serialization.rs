//! Serialization utilities for simulation state.

use crate::world::Snapshot;

/// Serialize a snapshot to JSON bytes.
pub fn snapshot_to_json(snapshot: &Snapshot) -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(snapshot)
}

/// Serialize a snapshot to a JSON string.
pub fn snapshot_to_json_string(snapshot: &Snapshot) -> Result<String, serde_json::Error> {
    serde_json::to_string(snapshot)
}

/// Deserialize a snapshot from JSON bytes.
pub fn snapshot_from_json(data: &[u8]) -> Result<Snapshot, serde_json::Error> {
    serde_json::from_slice(data)
}

/// Deserialize a snapshot from a JSON string.
pub fn snapshot_from_json_string(data: &str) -> Result<Snapshot, serde_json::Error> {
    serde_json::from_str(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::SquadSnapshot;

    #[test]
    fn test_snapshot_roundtrip() {
        let snapshot = Snapshot {
            tick: 42,
            time: 2.1,
            squads: vec![SquadSnapshot {
                id: 1,
                faction: "Blue".to_string(),
                x: 10.0,
                y: 20.0,
                vx: 1.0,
                vy: 0.0,
                health: 100.0,
                health_max: 100.0,
                size: 12,
                morale: 1.0,
                suppression: 0.0,
                order: "Hold".to_string(),
            }],
            destructibles: vec![],
            terrain_damage: vec![],
            new_craters: vec![],
            terrain_dirty: false,
        };

        let json = snapshot_to_json_string(&snapshot).unwrap();
        let restored = snapshot_from_json_string(&json).unwrap();

        assert_eq!(restored.tick, 42);
        assert_eq!(restored.squads.len(), 1);
        assert_eq!(restored.squads[0].id, 1);
    }
}
