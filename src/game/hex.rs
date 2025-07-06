use bevy::prelude::*;

/// Axial coordinates for hex grid (q, r)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Convert hex coordinates to world position (flat-top orientation)
    pub fn to_world_pos(self, hex_size: f32) -> Vec2 {
        let x = hex_size * (3.0_f32.sqrt() * self.q as f32 + 3.0_f32.sqrt() / 2.0 * self.r as f32);
        let y = hex_size * (3.0 / 2.0 * self.r as f32);
        Vec2::new(x, y)
    }

    /// Convert world position to hex coordinates
    pub fn from_world_pos(world_pos: Vec2, hex_size: f32) -> Self {
        // Convert to fractional axial coordinates
        let q = (3.0_f32.sqrt() / 3.0 * world_pos.x - 1.0 / 3.0 * world_pos.y) / hex_size;
        let r = (2.0 / 3.0 * world_pos.y) / hex_size;
        
        // Round to nearest hex
        Self::round_hex(q, r)
    }

    /// Round fractional hex coordinates to nearest hex
    fn round_hex(q: f32, r: f32) -> Self {
        let s = -q - r; // cube coordinate
        
        let mut rq = q.round();
        let mut rr = r.round();
        let rs = s.round();
        
        let q_diff = (rq - q).abs();
        let r_diff = (rr - r).abs();
        let s_diff = (rs - s).abs();
        
        if q_diff > r_diff && q_diff > s_diff {
            rq = -rr - rs;
        } else if r_diff > s_diff {
            rr = -rq - rs;
        }
        
        Self::new(rq as i32, rr as i32)
    }

    /// Get the 6 neighboring hex coordinates
    pub fn neighbors(self) -> [HexCoord; 6] {
        let directions = [
            (1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)
        ];
        
        let mut neighbors = [HexCoord::new(0, 0); 6];
        for (i, (dq, dr)) in directions.iter().enumerate() {
            neighbors[i] = HexCoord::new(self.q + dq, self.r + dr);
        }
        neighbors
    }
}