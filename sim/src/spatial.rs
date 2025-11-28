//! Spatial partitioning for efficient neighbor queries.
//!
//! Provides O(1) cell lookup and O(k) neighbor queries where k is the number
//! of entities in nearby cells, rather than O(n) for brute force.

use bevy_ecs::prelude::*;
use std::collections::HashMap;

/// Grid-based spatial partitioning structure.
/// 
/// Divides the world into cells and tracks which entities are in each cell.
/// Enables fast neighbor queries by only checking nearby cells.
#[derive(Resource, Debug)]
pub struct SpatialGrid {
    /// Cell size in world units.
    pub cell_size: f32,
    /// Map from cell coordinates to list of entities in that cell.
    cells: HashMap<(i32, i32), Vec<SpatialEntry>>,
    /// Reverse lookup: entity to cell.
    entity_cells: HashMap<Entity, (i32, i32)>,
}

/// Entry in a spatial cell.
#[derive(Debug, Clone, Copy)]
pub struct SpatialEntry {
    pub entity: Entity,
    pub x: f32,
    pub y: f32,
    pub faction: u8, // 0 = Blue, 1 = Red, 2 = Neutral
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self::new(20.0) // 20 unit cells by default
    }
}

impl SpatialGrid {
    /// Create a new spatial grid with the given cell size.
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
            entity_cells: HashMap::new(),
        }
    }

    /// Convert world coordinates to cell coordinates.
    #[inline]
    pub fn world_to_cell(&self, x: f32, y: f32) -> (i32, i32) {
        (
            (x / self.cell_size).floor() as i32,
            (y / self.cell_size).floor() as i32,
        )
    }

    /// Clear all entries (call at start of each frame before rebuilding).
    pub fn clear(&mut self) {
        self.cells.clear();
        self.entity_cells.clear();
    }

    /// Insert an entity at a position.
    pub fn insert(&mut self, entity: Entity, x: f32, y: f32, faction: u8) {
        let cell = self.world_to_cell(x, y);
        
        // Remove from old cell if moved
        if let Some(&old_cell) = self.entity_cells.get(&entity) {
            if old_cell != cell {
                if let Some(entries) = self.cells.get_mut(&old_cell) {
                    entries.retain(|e| e.entity != entity);
                }
            }
        }

        // Add to new cell
        let entry = SpatialEntry { entity, x, y, faction };
        self.cells.entry(cell).or_default().push(entry);
        self.entity_cells.insert(entity, cell);
    }

    /// Remove an entity from the grid.
    pub fn remove(&mut self, entity: Entity) {
        if let Some(cell) = self.entity_cells.remove(&entity) {
            if let Some(entries) = self.cells.get_mut(&cell) {
                entries.retain(|e| e.entity != entity);
            }
        }
    }

    /// Query all entities within a radius of a point.
    /// Returns entries sorted by distance (closest first).
    pub fn query_radius(&self, x: f32, y: f32, radius: f32) -> Vec<SpatialEntry> {
        let radius_sq = radius * radius;
        let cells_to_check = (radius / self.cell_size).ceil() as i32 + 1;
        let center_cell = self.world_to_cell(x, y);
        
        let mut results = Vec::new();

        for dx in -cells_to_check..=cells_to_check {
            for dy in -cells_to_check..=cells_to_check {
                let cell = (center_cell.0 + dx, center_cell.1 + dy);
                if let Some(entries) = self.cells.get(&cell) {
                    for entry in entries {
                        let dist_sq = (entry.x - x).powi(2) + (entry.y - y).powi(2);
                        if dist_sq <= radius_sq {
                            results.push(*entry);
                        }
                    }
                }
            }
        }

        // Sort by distance
        results.sort_by(|a, b| {
            let dist_a = (a.x - x).powi(2) + (a.y - y).powi(2);
            let dist_b = (b.x - x).powi(2) + (b.y - y).powi(2);
            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Query all entities within a radius, filtering by faction.
    /// faction_filter: None = all, Some(f) = only faction f
    pub fn query_radius_faction(
        &self,
        x: f32,
        y: f32,
        radius: f32,
        faction_filter: Option<u8>,
    ) -> Vec<SpatialEntry> {
        let mut results = self.query_radius(x, y, radius);
        if let Some(faction) = faction_filter {
            results.retain(|e| e.faction == faction);
        }
        results
    }

    /// Query enemies within radius (faction != given faction).
    pub fn query_enemies(&self, x: f32, y: f32, radius: f32, my_faction: u8) -> Vec<SpatialEntry> {
        let mut results = self.query_radius(x, y, radius);
        results.retain(|e| e.faction != my_faction);
        results
    }

    /// Query friendlies within radius (faction == given faction).
    pub fn query_friendlies(&self, x: f32, y: f32, radius: f32, my_faction: u8) -> Vec<SpatialEntry> {
        let mut results = self.query_radius(x, y, radius);
        results.retain(|e| e.faction == my_faction);
        results
    }

    /// Get the nearest enemy to a position.
    pub fn nearest_enemy(&self, x: f32, y: f32, max_radius: f32, my_faction: u8) -> Option<SpatialEntry> {
        self.query_enemies(x, y, max_radius, my_faction).into_iter().next()
    }

    /// Get count of entities in a cell.
    pub fn cell_count(&self, cell: (i32, i32)) -> usize {
        self.cells.get(&cell).map(|v| v.len()).unwrap_or(0)
    }

    /// Get total entity count.
    pub fn total_count(&self) -> usize {
        self.entity_cells.len()
    }

    /// Get all cells (for debugging/visualization).
    pub fn all_cells(&self) -> impl Iterator<Item = (&(i32, i32), &Vec<SpatialEntry>)> {
        self.cells.iter()
    }
}

/// System that rebuilds the spatial grid each frame.
pub fn spatial_grid_update_system(
    mut grid: ResMut<SpatialGrid>,
    query: Query<(Entity, &crate::components::Position, &crate::components::Faction, &crate::components::Health)>,
) {
    grid.clear();
    
    for (entity, pos, faction, health) in query.iter() {
        if !health.is_alive() {
            continue;
        }
        
        let faction_id = match faction {
            crate::components::Faction::Blue => 0,
            crate::components::Faction::Red => 1,
        };
        
        grid.insert(entity, pos.x, pos.y, faction_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spatial_grid_insert_query() {
        let mut grid = SpatialGrid::new(10.0);
        
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);
        
        grid.insert(e1, 5.0, 5.0, 0);
        grid.insert(e2, 15.0, 5.0, 0);
        grid.insert(e3, 100.0, 100.0, 1);
        
        // Query around e1
        let nearby = grid.query_radius(5.0, 5.0, 15.0);
        assert_eq!(nearby.len(), 2); // e1 and e2
        
        // Query with smaller radius
        let nearby = grid.query_radius(5.0, 5.0, 5.0);
        assert_eq!(nearby.len(), 1); // just e1
        
        // Query far away
        let nearby = grid.query_radius(100.0, 100.0, 10.0);
        assert_eq!(nearby.len(), 1); // just e3
    }

    #[test]
    fn test_faction_queries() {
        let mut grid = SpatialGrid::new(10.0);
        
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);
        
        grid.insert(e1, 0.0, 0.0, 0); // Blue
        grid.insert(e2, 5.0, 0.0, 0); // Blue
        grid.insert(e3, 10.0, 0.0, 1); // Red
        
        // Query enemies from Blue perspective
        let enemies = grid.query_enemies(0.0, 0.0, 20.0, 0);
        assert_eq!(enemies.len(), 1);
        assert_eq!(enemies[0].faction, 1);
        
        // Query friendlies from Blue perspective
        let friends = grid.query_friendlies(0.0, 0.0, 20.0, 0);
        assert_eq!(friends.len(), 2);
    }

    #[test]
    fn test_nearest_enemy() {
        let mut grid = SpatialGrid::new(10.0);
        
        let e1 = Entity::from_raw(1);
        let e2 = Entity::from_raw(2);
        let e3 = Entity::from_raw(3);
        
        grid.insert(e1, 0.0, 0.0, 0); // Blue at origin
        grid.insert(e2, 30.0, 0.0, 1); // Red at 30
        grid.insert(e3, 20.0, 0.0, 1); // Red at 20 (closer)
        
        let nearest = grid.nearest_enemy(0.0, 0.0, 50.0, 0);
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().entity, e3); // e3 is closer
    }
}
