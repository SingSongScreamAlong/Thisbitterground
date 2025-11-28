//! Terrain system - heightmap, craters, and terrain effects.
//!
//! The terrain is represented as a grid-based heightmap that can be deformed
//! by explosions, artillery, and other effects. Terrain affects movement speed
//! and provides cover bonuses.

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Resource wrapper for terrain grid, allowing shared access in ECS systems.
#[derive(Resource, Clone)]
pub struct TerrainResource(pub Arc<std::sync::RwLock<TerrainGrid>>);

impl TerrainResource {
    pub fn new(grid: TerrainGrid) -> Self {
        Self(Arc::new(std::sync::RwLock::new(grid)))
    }

    /// Get movement multiplier at a position (read-only access).
    pub fn get_movement_multiplier(&self, x: f32, y: f32) -> f32 {
        self.0.read().map(|g| g.get_movement_multiplier(x, y)).unwrap_or(1.0)
    }

    /// Get cover value at a position (read-only access).
    pub fn get_cover_at(&self, x: f32, y: f32) -> f32 {
        self.0.read().map(|g| g.get_cover_at(x, y)).unwrap_or(0.0)
    }

    /// Get height at a position (read-only access).
    pub fn get_height_at(&self, x: f32, y: f32) -> f32 {
        self.0.read().map(|g| g.get_height_at(x, y)).unwrap_or(0.0)
    }
}

/// Terrain type at a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainType {
    /// Open ground - normal movement, no cover.
    Open,
    /// Rough terrain - slower movement, light cover.
    Rough,
    /// Mud - very slow movement, no cover.
    Mud,
    /// Crater - slow movement, good cover.
    Crater,
    /// Trench - normal movement, excellent cover.
    Trench,
    /// Water - impassable or very slow.
    Water,
    /// Road - fast movement, no cover.
    Road,
    /// Forest - slow movement, good cover (until destroyed).
    Forest,
    /// Rubble - slow movement, moderate cover.
    Rubble,
}

impl Default for TerrainType {
    fn default() -> Self {
        Self::Open
    }
}

impl TerrainType {
    /// Movement speed multiplier for this terrain type.
    pub fn movement_multiplier(&self) -> f32 {
        match self {
            TerrainType::Open => 1.0,
            TerrainType::Rough => 0.7,
            TerrainType::Mud => 0.4,
            TerrainType::Crater => 0.6,
            TerrainType::Trench => 0.9,
            TerrainType::Water => 0.2,
            TerrainType::Road => 1.3,
            TerrainType::Forest => 0.6,
            TerrainType::Rubble => 0.5,
        }
    }

    /// Cover value provided by this terrain (0.0 = none, 1.0 = full).
    pub fn cover_value(&self) -> f32 {
        match self {
            TerrainType::Open => 0.0,
            TerrainType::Rough => 0.2,
            TerrainType::Mud => 0.0,
            TerrainType::Crater => 0.5,
            TerrainType::Trench => 0.8,
            TerrainType::Water => 0.0,
            TerrainType::Road => 0.0,
            TerrainType::Forest => 0.4,
            TerrainType::Rubble => 0.3,
        }
    }

    /// Whether this terrain blocks line of sight.
    pub fn blocks_los(&self) -> bool {
        matches!(self, TerrainType::Forest)
    }
}

/// A single cell in the terrain grid.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TerrainCell {
    /// Height at this cell (0.0 = sea level).
    pub height: f32,
    /// Type of terrain at this cell.
    pub terrain_type: TerrainType,
    /// Accumulated damage/deformation at this cell.
    pub damage: f32,
}

impl Default for TerrainCell {
    fn default() -> Self {
        Self {
            height: 0.0,
            terrain_type: TerrainType::Open,
            damage: 0.0,
        }
    }
}

/// Grid-based terrain heightmap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainGrid {
    /// Width of the grid in cells.
    pub width: usize,
    /// Height of the grid in cells.
    pub height: usize,
    /// Size of each cell in world units.
    pub cell_size: f32,
    /// Origin offset (world position of cell 0,0).
    pub origin_x: f32,
    pub origin_y: f32,
    /// Grid cells (row-major order).
    pub cells: Vec<TerrainCell>,
    /// List of craters for visualization.
    pub craters: Vec<Crater>,
}

/// A crater in the terrain.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Crater {
    /// World position X.
    pub x: f32,
    /// World position Y.
    pub y: f32,
    /// Radius of the crater.
    pub radius: f32,
    /// Depth of the crater.
    pub depth: f32,
    /// Age of the crater in simulation time.
    pub age: f32,
}

impl TerrainGrid {
    /// Create a new terrain grid.
    pub fn new(width: usize, height: usize, cell_size: f32) -> Self {
        let origin_x = -(width as f32 * cell_size) / 2.0;
        let origin_y = -(height as f32 * cell_size) / 2.0;
        
        Self {
            width,
            height,
            cell_size,
            origin_x,
            origin_y,
            cells: vec![TerrainCell::default(); width * height],
            craters: Vec::new(),
        }
    }

    /// Create a terrain grid with some initial features.
    pub fn new_with_features(width: usize, height: usize, cell_size: f32) -> Self {
        let mut grid = Self::new(width, height, cell_size);
        
        // Add some random terrain features
        // Central road (horizontal)
        let mid_y = height / 2;
        for x in 0..width {
            if let Some(cell) = grid.get_cell_mut(x, mid_y) {
                cell.terrain_type = TerrainType::Road;
            }
            if mid_y > 0 {
                if let Some(cell) = grid.get_cell_mut(x, mid_y - 1) {
                    cell.terrain_type = TerrainType::Road;
                }
            }
        }
        
        // Add some forest patches
        grid.add_forest_patch(width / 4, height / 4, 5);
        grid.add_forest_patch(3 * width / 4, height / 4, 4);
        grid.add_forest_patch(width / 4, 3 * height / 4, 4);
        grid.add_forest_patch(3 * width / 4, 3 * height / 4, 5);
        
        // Add some rough terrain
        grid.add_rough_patch(width / 3, height / 3, 3);
        grid.add_rough_patch(2 * width / 3, 2 * height / 3, 3);
        
        grid
    }

    fn add_forest_patch(&mut self, cx: usize, cy: usize, radius: usize) {
        for dy in 0..=radius * 2 {
            for dx in 0..=radius * 2 {
                let x = cx.saturating_sub(radius) + dx;
                let y = cy.saturating_sub(radius) + dy;
                
                let dist_sq = (x as i32 - cx as i32).pow(2) + (y as i32 - cy as i32).pow(2);
                if dist_sq <= (radius as i32).pow(2) {
                    if let Some(cell) = self.get_cell_mut(x, y) {
                        cell.terrain_type = TerrainType::Forest;
                    }
                }
            }
        }
    }

    fn add_rough_patch(&mut self, cx: usize, cy: usize, radius: usize) {
        for dy in 0..=radius * 2 {
            for dx in 0..=radius * 2 {
                let x = cx.saturating_sub(radius) + dx;
                let y = cy.saturating_sub(radius) + dy;
                
                let dist_sq = (x as i32 - cx as i32).pow(2) + (y as i32 - cy as i32).pow(2);
                if dist_sq <= (radius as i32).pow(2) {
                    if let Some(cell) = self.get_cell_mut(x, y) {
                        if cell.terrain_type == TerrainType::Open {
                            cell.terrain_type = TerrainType::Rough;
                        }
                    }
                }
            }
        }
    }

    /// Get the cell index for grid coordinates.
    fn cell_index(&self, x: usize, y: usize) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y * self.width + x)
        } else {
            None
        }
    }

    /// Get a cell by grid coordinates.
    pub fn get_cell(&self, x: usize, y: usize) -> Option<&TerrainCell> {
        self.cell_index(x, y).map(|i| &self.cells[i])
    }

    /// Get a mutable cell by grid coordinates.
    pub fn get_cell_mut(&mut self, x: usize, y: usize) -> Option<&mut TerrainCell> {
        self.cell_index(x, y).map(|i| &mut self.cells[i])
    }

    /// Convert world coordinates to grid coordinates.
    pub fn world_to_grid(&self, world_x: f32, world_y: f32) -> (usize, usize) {
        let gx = ((world_x - self.origin_x) / self.cell_size).floor() as i32;
        let gy = ((world_y - self.origin_y) / self.cell_size).floor() as i32;
        
        let gx = gx.clamp(0, self.width as i32 - 1) as usize;
        let gy = gy.clamp(0, self.height as i32 - 1) as usize;
        
        (gx, gy)
    }

    /// Convert grid coordinates to world coordinates (center of cell).
    pub fn grid_to_world(&self, gx: usize, gy: usize) -> (f32, f32) {
        let world_x = self.origin_x + (gx as f32 + 0.5) * self.cell_size;
        let world_y = self.origin_y + (gy as f32 + 0.5) * self.cell_size;
        (world_x, world_y)
    }

    /// Get terrain info at a world position.
    pub fn get_terrain_at(&self, world_x: f32, world_y: f32) -> TerrainCell {
        let (gx, gy) = self.world_to_grid(world_x, world_y);
        self.get_cell(gx, gy).copied().unwrap_or_default()
    }

    /// Get height at a world position (with interpolation).
    pub fn get_height_at(&self, world_x: f32, world_y: f32) -> f32 {
        // Simple nearest-neighbor for now
        let cell = self.get_terrain_at(world_x, world_y);
        cell.height
    }

    /// Get movement multiplier at a world position.
    pub fn get_movement_multiplier(&self, world_x: f32, world_y: f32) -> f32 {
        let cell = self.get_terrain_at(world_x, world_y);
        cell.terrain_type.movement_multiplier()
    }

    /// Get cover value at a world position.
    pub fn get_cover_at(&self, world_x: f32, world_y: f32) -> f32 {
        let cell = self.get_terrain_at(world_x, world_y);
        cell.terrain_type.cover_value()
    }

    /// Apply a crater/explosion to the terrain.
    pub fn apply_crater(&mut self, world_x: f32, world_y: f32, radius: f32, depth: f32) {
        let (cx, cy) = self.world_to_grid(world_x, world_y);
        let grid_radius = (radius / self.cell_size).ceil() as i32;
        
        // Pre-compute cell positions to avoid borrow issues
        let origin_x = self.origin_x;
        let origin_y = self.origin_y;
        let cell_size = self.cell_size;
        
        // Deform terrain in radius
        for dy in -grid_radius..=grid_radius {
            for dx in -grid_radius..=grid_radius {
                let gx = (cx as i32 + dx) as usize;
                let gy = (cy as i32 + dy) as usize;
                
                // Compute cell world position inline
                let cell_x = origin_x + (gx as f32 + 0.5) * cell_size;
                let cell_y = origin_y + (gy as f32 + 0.5) * cell_size;
                let dist = ((cell_x - world_x).powi(2) + (cell_y - world_y).powi(2)).sqrt();
                
                if dist <= radius {
                    if let Some(cell) = self.get_cell_mut(gx, gy) {
                        // Crater depth falloff
                        let falloff = 1.0 - (dist / radius);
                        let height_change = -depth * falloff * falloff;
                        
                        cell.height += height_change;
                        cell.damage += depth * falloff;
                        
                        // Convert terrain to crater if heavily damaged
                        if cell.damage > 2.0 {
                            cell.terrain_type = TerrainType::Crater;
                        } else if cell.damage > 1.0 && cell.terrain_type == TerrainType::Forest {
                            cell.terrain_type = TerrainType::Rubble;
                        } else if cell.damage > 0.5 && cell.terrain_type == TerrainType::Open {
                            cell.terrain_type = TerrainType::Rough;
                        }
                    }
                }
            }
        }
        
        // Add crater to list for visualization
        self.craters.push(Crater {
            x: world_x,
            y: world_y,
            radius,
            depth,
            age: 0.0,
        });
    }

    /// Apply artillery barrage to an area.
    pub fn apply_barrage(&mut self, center_x: f32, center_y: f32, spread: f32, count: usize, crater_radius: f32, crater_depth: f32) {
        use std::f32::consts::PI;
        
        // Simple pseudo-random scatter
        for i in 0..count {
            let angle = (i as f32 / count as f32) * 2.0 * PI + (i as f32 * 1.618);
            let dist = spread * (0.3 + 0.7 * ((i as f32 * 0.7).sin().abs()));
            
            let x = center_x + dist * angle.cos();
            let y = center_y + dist * angle.sin();
            
            self.apply_crater(x, y, crater_radius, crater_depth);
        }
    }

    /// Update crater ages.
    pub fn update(&mut self, dt: f32) {
        for crater in &mut self.craters {
            crater.age += dt;
        }
    }

    /// Get world bounds.
    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        let min_x = self.origin_x;
        let min_y = self.origin_y;
        let max_x = self.origin_x + self.width as f32 * self.cell_size;
        let max_y = self.origin_y + self.height as f32 * self.cell_size;
        (min_x, min_y, max_x, max_y)
    }
}

/// Snapshot of terrain for serialization to Godot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerrainSnapshot {
    pub width: usize,
    pub height: usize,
    pub cell_size: f32,
    pub origin_x: f32,
    pub origin_y: f32,
    /// Flattened height data.
    pub heights: Vec<f32>,
    /// Flattened terrain type data (as u8).
    pub types: Vec<u8>,
    /// Active craters.
    pub craters: Vec<Crater>,
}

impl TerrainSnapshot {
    pub fn from_grid(grid: &TerrainGrid) -> Self {
        let heights: Vec<f32> = grid.cells.iter().map(|c| c.height).collect();
        let types: Vec<u8> = grid.cells.iter().map(|c| c.terrain_type as u8).collect();
        
        Self {
            width: grid.width,
            height: grid.height,
            cell_size: grid.cell_size,
            origin_x: grid.origin_x,
            origin_y: grid.origin_y,
            heights,
            types,
            craters: grid.craters.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_grid_creation() {
        let grid = TerrainGrid::new(100, 100, 2.0);
        assert_eq!(grid.width, 100);
        assert_eq!(grid.height, 100);
        assert_eq!(grid.cells.len(), 10000);
    }

    #[test]
    fn test_world_to_grid() {
        let grid = TerrainGrid::new(100, 100, 2.0);
        // Origin should be at (-100, -100)
        let (gx, gy) = grid.world_to_grid(0.0, 0.0);
        assert_eq!(gx, 50);
        assert_eq!(gy, 50);
    }

    #[test]
    fn test_crater_application() {
        let mut grid = TerrainGrid::new(50, 50, 2.0);
        grid.apply_crater(0.0, 0.0, 5.0, 2.0);
        
        assert_eq!(grid.craters.len(), 1);
        
        let cell = grid.get_terrain_at(0.0, 0.0);
        assert!(cell.height < 0.0);
        assert!(cell.damage > 0.0);
    }

    #[test]
    fn test_movement_multiplier() {
        assert_eq!(TerrainType::Open.movement_multiplier(), 1.0);
        assert!(TerrainType::Mud.movement_multiplier() < 1.0);
        assert!(TerrainType::Road.movement_multiplier() > 1.0);
    }

    #[test]
    fn test_cover_values() {
        assert_eq!(TerrainType::Open.cover_value(), 0.0);
        assert!(TerrainType::Crater.cover_value() > 0.0);
        assert!(TerrainType::Trench.cover_value() > TerrainType::Crater.cover_value());
    }
}
