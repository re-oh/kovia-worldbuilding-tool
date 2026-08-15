use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    brush::BrushFalloff,
    coordinates::TileId,
    history::{EditHistory, UndoAction},
};

pub const DEFAULT_TILE_CELLS: u16 = 128;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeightTile {
    pub id: TileId,
    /// A 128-cell tile has 129 samples so adjacent tiles can share an edge.
    pub edge_samples: u16,
    pub revision: u64,
    pub min_height_m: f32,
    pub max_height_m: f32,
    samples: Vec<f32>,
}

impl HeightTile {
    pub fn flat(id: TileId, edge_samples: u16, height_m: f32) -> Self {
        assert!(edge_samples >= 2);
        Self {
            id,
            edge_samples,
            revision: 0,
            min_height_m: height_m,
            max_height_m: height_m,
            samples: vec![height_m; edge_samples as usize * edge_samples as usize],
        }
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn edge_cells(&self) -> usize {
        self.edge_samples as usize - 1
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        debug_assert!(x < self.edge_samples as usize && y < self.edge_samples as usize);
        y * self.edge_samples as usize + x
    }

    pub fn get(&self, x: usize, y: usize) -> f32 {
        self.samples[self.index(x, y)]
    }

    pub fn set(&mut self, x: usize, y: usize, height_m: f32) {
        let index = self.index(x, y);
        self.samples[index] = height_m;
    }

    pub fn sample_bilinear(&self, uv: [f32; 2]) -> f32 {
        let max = self.edge_cells() as f32;
        let x = (uv[0].clamp(0.0, 1.0) * max).clamp(0.0, max);
        let y = (uv[1].clamp(0.0, 1.0) * max).clamp(0.0, max);
        let x0 = x.floor() as usize;
        let y0 = y.floor() as usize;
        let x1 = (x0 + 1).min(self.edge_cells());
        let y1 = (y0 + 1).min(self.edge_cells());
        let tx = x - x0 as f32;
        let ty = y - y0 as f32;
        let a = self.get(x0, y0) * (1.0 - tx) + self.get(x1, y0) * tx;
        let b = self.get(x0, y1) * (1.0 - tx) + self.get(x1, y1) * tx;
        a * (1.0 - ty) + b * ty
    }

    pub fn recompute_range(&mut self) {
        self.min_height_m = self.samples.iter().copied().fold(f32::INFINITY, f32::min);
        self.max_height_m = self
            .samples
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
    }

    pub fn touch(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.recompute_range();
    }
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct TerrainStore {
    /// Only authored or currently cached tiles exist in memory.
    pub tiles: BTreeMap<TileId, HeightTile>,
}

/// Public input port for terrain sculpting. UI, scripts, tests, and networked
/// tools all use the same request instead of reaching into `TerrainStore`.
#[derive(Debug, Clone, Event)]
pub struct SculptTerrain(pub HeightBrushStamp);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeightProposal {
    pub tile: TileId,
    pub source_revision: u64,
    pub samples: Vec<f32>,
}

#[derive(Debug, Clone, Event)]
pub struct ApplyHeightProposal(pub HeightProposal);

/// Output port published after canonical terrain has changed.
#[derive(Debug, Clone, Copy, Event)]
pub struct TerrainChanged {
    pub tile: TileId,
    pub revision: u64,
}

#[derive(Debug, Clone, Error)]
pub enum TerrainEditError {
    #[error("terrain tile {0:?} is not loaded")]
    MissingTile(TileId),
    #[error("computed terrain proposal is stale (expected revision {expected}, found {actual})")]
    StaleProposal { expected: u64, actual: u64 },
    #[error("computed terrain proposal has the wrong sample count")]
    InvalidProposalSize,
}

#[derive(Debug, Clone, Event)]
pub struct TerrainEditFailed(pub TerrainEditError);

/// Authoritative terrain data and terrain edits. No camera, UI, layer,
/// hydrology, or rendering concerns are installed here.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainStore>()
            .add_observer(apply_sculpt_request)
            .add_observer(apply_height_proposal);
    }
}

fn apply_sculpt_request(
    request: On<SculptTerrain>,
    mut terrain: ResMut<TerrainStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let stamp = request.0;
    let Some(tile) = terrain.tiles.get_mut(&stamp.tile) else {
        commands.trigger(TerrainEditFailed(TerrainEditError::MissingTile(stamp.tile)));
        return;
    };
    let patch = sculpt(tile, stamp);
    if patch.changes.is_empty() {
        return;
    }
    if let Some(history) = history.as_deref_mut() {
        history.record(HeightUndo(patch));
    }
    commands.trigger(TerrainChanged {
        tile: tile.id,
        revision: tile.revision,
    });
}

fn apply_height_proposal(
    request: On<ApplyHeightProposal>,
    mut terrain: ResMut<TerrainStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let proposal = &request.0;
    let Some(tile) = terrain.tiles.get_mut(&proposal.tile) else {
        commands.trigger(TerrainEditFailed(TerrainEditError::MissingTile(
            proposal.tile,
        )));
        return;
    };
    if tile.revision != proposal.source_revision {
        commands.trigger(TerrainEditFailed(TerrainEditError::StaleProposal {
            expected: proposal.source_revision,
            actual: tile.revision,
        }));
        return;
    }
    if tile.samples().len() != proposal.samples.len() {
        commands.trigger(TerrainEditFailed(TerrainEditError::InvalidProposalSize));
        return;
    }
    let changes = tile
        .samples()
        .iter()
        .zip(&proposal.samples)
        .enumerate()
        .filter_map(|(index, (&before, &after))| {
            (before != after).then_some(HeightChange {
                index: index as u32,
                before,
                after,
            })
        })
        .collect();
    let patch = HeightPatch {
        tile: proposal.tile,
        changes,
    };
    if patch.changes.is_empty() {
        return;
    }
    apply_height_patch(tile, &patch, true);
    if let Some(history) = history.as_deref_mut() {
        history.record(HeightUndo(patch));
    }
    commands.trigger(TerrainChanged {
        tile: tile.id,
        revision: tile.revision,
    });
}

impl TerrainStore {
    pub fn ensure_flat(&mut self, id: TileId, edge_samples: u16, height_m: f32) -> &mut HeightTile {
        self.tiles
            .entry(id)
            .or_insert_with(|| HeightTile::flat(id, edge_samples, height_m))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SculptMode {
    Raise,
    Lower,
    Flatten { target_m: f32 },
    Smooth,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeightBrushStamp {
    pub tile: TileId,
    /// Tile-local coordinate in `0..=1`.
    pub center_uv: [f32; 2],
    /// Radius measured in cells at this tile's level.
    pub radius_cells: f32,
    /// Metres per stamp for raise/lower; blend amount for flatten/smooth.
    pub strength: f32,
    pub falloff: BrushFalloff,
    pub mode: SculptMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HeightChange {
    pub index: u32,
    pub before: f32,
    pub after: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeightPatch {
    pub tile: TileId,
    pub changes: Vec<HeightChange>,
}

struct HeightUndo(HeightPatch);

impl UndoAction for HeightUndo {
    fn undo(&self, world: &mut World) -> Result<(), String> {
        apply_history_patch(world, &self.0, false)
    }

    fn redo(&self, world: &mut World) -> Result<(), String> {
        apply_history_patch(world, &self.0, true)
    }
}

fn apply_history_patch(
    world: &mut World,
    patch: &HeightPatch,
    forward: bool,
) -> Result<(), String> {
    let revision = {
        let mut terrain = world
            .get_resource_mut::<TerrainStore>()
            .ok_or_else(|| "terrain plugin is not installed".to_string())?;
        let tile = terrain
            .tiles
            .get_mut(&patch.tile)
            .ok_or_else(|| "terrain tile needed by history is not resident".to_string())?;
        apply_height_patch(tile, patch, forward);
        tile.revision
    };
    world.trigger(TerrainChanged {
        tile: patch.tile,
        revision,
    });
    Ok(())
}

pub fn sculpt(tile: &mut HeightTile, stamp: HeightBrushStamp) -> HeightPatch {
    let source = tile.samples.clone();
    let edge = tile.edge_samples as usize;
    let cells = tile.edge_cells() as f32;
    let center = [stamp.center_uv[0] * cells, stamp.center_uv[1] * cells];
    let radius = stamp.radius_cells.max(0.5);
    let min_x = (center[0] - radius).floor().max(0.0) as usize;
    let min_y = (center[1] - radius).floor().max(0.0) as usize;
    let max_x = (center[0] + radius).ceil().min((edge - 1) as f32) as usize;
    let max_y = (center[1] + radius).ceil().min((edge - 1) as f32) as usize;
    let mut changes = Vec::new();

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 - center[0];
            let dy = y as f32 - center[1];
            let distance = (dx * dx + dy * dy).sqrt();
            if distance > radius {
                continue;
            }
            let weight = stamp.falloff.weight(distance / radius);
            let index = tile.index(x, y);
            let before = source[index];
            let after = match stamp.mode {
                SculptMode::Raise => before + stamp.strength * weight,
                SculptMode::Lower => before - stamp.strength * weight,
                SculptMode::Flatten { target_m } => {
                    before + (target_m - before) * (stamp.strength * weight).clamp(0.0, 1.0)
                }
                SculptMode::Smooth => {
                    let mut sum = 0.0;
                    let mut count = 0.0;
                    for sy in y.saturating_sub(1)..=(y + 1).min(edge - 1) {
                        for sx in x.saturating_sub(1)..=(x + 1).min(edge - 1) {
                            sum += source[sy * edge + sx];
                            count += 1.0;
                        }
                    }
                    let average = sum / count;
                    before + (average - before) * (stamp.strength * weight).clamp(0.0, 1.0)
                }
            };
            if after != before {
                tile.samples[index] = after;
                changes.push(HeightChange {
                    index: index as u32,
                    before,
                    after,
                });
            }
        }
    }

    if !changes.is_empty() {
        tile.touch();
    }
    HeightPatch {
        tile: tile.id,
        changes,
    }
}

pub fn apply_height_patch(tile: &mut HeightTile, patch: &HeightPatch, forward: bool) {
    debug_assert_eq!(tile.id, patch.tile);
    for change in &patch.changes {
        tile.samples[change.index as usize] = if forward { change.after } else { change.before };
    }
    if !patch.changes.is_empty() {
        tile.touch();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sculpt_patch_is_reversible() {
        let mut tile = HeightTile::flat(TileId::ROOT, 9, 0.0);
        let original = tile.samples.clone();
        let patch = sculpt(
            &mut tile,
            HeightBrushStamp {
                tile: TileId::ROOT,
                center_uv: [0.5, 0.5],
                radius_cells: 2.0,
                strength: 100.0,
                falloff: BrushFalloff::Smooth,
                mode: SculptMode::Raise,
            },
        );
        assert!(tile.max_height_m > 0.0);
        apply_height_patch(&mut tile, &patch, false);
        assert_eq!(tile.samples, original);
    }
}
