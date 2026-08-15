use bevy::prelude::{App, Plugin, Resource};
use serde::{Deserialize, Serialize};

/// A cube-sphere avoids a singularity at either pole and gives the planet six
/// identical quadtree roots. Rendering can begin as a flat face while the data
/// model remains planet-ready.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CubeFace {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

/// Stable address of a terrain tile. `x` and `y` are in `0..2^level`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TileId {
    pub face: CubeFace,
    pub level: u8,
    pub x: u32,
    pub y: u32,
}

impl TileId {
    pub const ROOT: Self = Self {
        face: CubeFace::PositiveY,
        level: 0,
        x: 0,
        y: 0,
    };

    pub fn new(face: CubeFace, level: u8, x: u32, y: u32) -> Option<Self> {
        let width = 1_u32.checked_shl(level.into())?;
        (x < width && y < width).then_some(Self { face, level, x, y })
    }

    pub fn width(self) -> u32 {
        1_u32 << self.level
    }

    pub fn parent(self) -> Option<Self> {
        (self.level > 0).then_some(Self {
            face: self.face,
            level: self.level.saturating_sub(1),
            x: self.x / 2,
            y: self.y / 2,
        })
    }

    pub fn children(self) -> [Self; 4] {
        let level = self.level + 1;
        let x = self.x * 2;
        let y = self.y * 2;
        [
            Self {
                face: self.face,
                level,
                x,
                y,
            },
            Self {
                face: self.face,
                level,
                x: x + 1,
                y,
            },
            Self {
                face: self.face,
                level,
                x,
                y: y + 1,
            },
            Self {
                face: self.face,
                level,
                x: x + 1,
                y: y + 1,
            },
        ]
    }

    /// Bounds on the cube face in the normalized interval `[-1, 1]`.
    pub fn face_uv_bounds(self) -> ([f64; 2], [f64; 2]) {
        let width = self.width() as f64;
        let size = 2.0 / width;
        let min = [-1.0 + self.x as f64 * size, -1.0 + self.y as f64 * size];
        (min, [min[0] + size, min[1] + size])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfacePosition {
    pub face: CubeFace,
    /// Cube-face coordinate in `[-1, 1]`.
    pub u: f64,
    /// Cube-face coordinate in `[-1, 1]`.
    pub v: f64,
    pub altitude_m: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Resource, Serialize, Deserialize)]
pub struct PlanetSpec {
    pub radius_m: f64,
    pub sea_level_m: f32,
}

/// Owns only the stable spatial reference for the world. It can be used with
/// none of the editor, renderer, terrain, or simulation plugins installed.
pub struct PlanetPlugin;

impl Plugin for PlanetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlanetSpec>();
    }
}

impl Default for PlanetSpec {
    fn default() -> Self {
        Self {
            radius_m: 6_000_000.0,
            sea_level_m: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadtree_addresses_round_trip_to_parent() {
        let tile = TileId::new(CubeFace::PositiveZ, 3, 5, 2).unwrap();
        let parent = tile.parent().unwrap();
        assert!(parent.children().contains(&tile));
    }

    #[test]
    fn root_covers_entire_face() {
        assert_eq!(TileId::ROOT.face_uv_bounds(), ([-1.0, -1.0], [1.0, 1.0]));
    }
}
