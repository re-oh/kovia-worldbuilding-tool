use std::collections::{BTreeMap, HashMap};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use kovia_protocol::{LayerId, RegionCode};

use crate::{
    brush::BrushFalloff,
    coordinates::TileId,
    history::{EditHistory, UndoAction},
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    Categorical,
    Scalar { minimum: f32, maximum: f32 },
    BinaryMask,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionDefinition {
    pub code: RegionCode,
    pub name: String,
    pub color_rgba: [u8; 4],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerDefinition {
    pub id: LayerId,
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    pub locked: bool,
    pub opacity: f32,
    pub regions: BTreeMap<RegionCode, RegionDefinition>,
    next_region_code: u16,
}

impl LayerDefinition {
    pub fn categorical(name: impl Into<String>) -> Self {
        Self {
            id: LayerId::new(),
            name: name.into(),
            kind: LayerKind::Categorical,
            visible: true,
            locked: false,
            opacity: 0.65,
            regions: BTreeMap::new(),
            next_region_code: 1,
        }
    }

    pub fn add_region(&mut self, name: impl Into<String>, color_rgba: [u8; 4]) -> RegionCode {
        let code = RegionCode(self.next_region_code);
        self.next_region_code = self
            .next_region_code
            .checked_add(1)
            .expect("region code space exhausted");
        self.regions.insert(
            code,
            RegionDefinition {
                code,
                name: name.into(),
                color_rgba,
            },
        );
        code
    }
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct LayerRegistry {
    pub ordering: Vec<LayerId>,
    pub definitions: BTreeMap<LayerId, LayerDefinition>,
}

impl LayerRegistry {
    pub fn insert(&mut self, layer: LayerDefinition) -> LayerId {
        let id = layer.id;
        self.ordering.push(id);
        self.definitions.insert(id, layer);
        id
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionTile {
    pub tile: TileId,
    pub edge_cells: u16,
    pub revision: u64,
    cells: Vec<RegionCode>,
}

impl RegionTile {
    pub fn empty(tile: TileId, edge_cells: u16) -> Self {
        Self {
            tile,
            edge_cells,
            revision: 0,
            cells: vec![RegionCode::UNASSIGNED; edge_cells as usize * edge_cells as usize],
        }
    }

    pub fn cells(&self) -> &[RegionCode] {
        &self.cells
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn index(&self, x: usize, y: usize) -> usize {
        y * self.edge_cells as usize + x
    }
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct LayerStore {
    pub categorical_tiles: HashMap<(LayerId, TileId), RegionTile>,
}

#[derive(Debug, Clone, Event)]
pub struct CreateLayer {
    pub name: String,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct LayerCreated {
    pub layer: LayerId,
}

#[derive(Debug, Clone, Event)]
pub struct CreateRegion {
    pub layer: LayerId,
    pub name: String,
    pub color_rgba: [u8; 4],
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RegionCreated {
    pub layer: LayerId,
    pub region: RegionCode,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct LayerDefinitionChanged {
    pub layer: LayerId,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct ConfigureLayer {
    pub layer: LayerId,
    pub visible: Option<bool>,
    pub locked: Option<bool>,
    pub opacity: Option<f32>,
}

/// `edge_cells` belongs to the request so semantic layers remain independent
/// from terrain storage. They can be painted over imported imagery or another
/// spatial substrate without installing `TerrainPlugin`.
#[derive(Debug, Clone, Copy, Event)]
pub struct PaintRegion {
    pub stamp: RegionBrushStamp,
    pub edge_cells: u16,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RegionChanged {
    pub layer: LayerId,
    pub tile: TileId,
    pub revision: u64,
}

#[derive(Debug, Clone, Error)]
pub enum LayerEditError {
    #[error("layer {0:?} does not exist")]
    MissingLayer(LayerId),
    #[error("layer {0:?} is locked")]
    LockedLayer(LayerId),
}

#[derive(Debug, Clone, Event)]
pub struct LayerEditFailed(pub LayerEditError);

/// Owns layer definitions, palettes, and packed categorical masks. It does not
/// require terrain, rendering, UI, or feature storage.
pub struct LayerPlugin;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LayerRegistry>()
            .init_resource::<LayerStore>()
            .add_observer(create_layer)
            .add_observer(create_region)
            .add_observer(configure_layer)
            .add_observer(paint_region_request);
    }
}

fn configure_layer(
    request: On<ConfigureLayer>,
    mut registry: ResMut<LayerRegistry>,
    mut commands: Commands,
) {
    let Some(layer) = registry.definitions.get_mut(&request.layer) else {
        commands.trigger(LayerEditFailed(LayerEditError::MissingLayer(request.layer)));
        return;
    };
    if let Some(visible) = request.visible {
        layer.visible = visible;
    }
    if let Some(locked) = request.locked {
        layer.locked = locked;
    }
    if let Some(opacity) = request.opacity {
        layer.opacity = opacity.clamp(0.0, 1.0);
    }
    commands.trigger(LayerDefinitionChanged {
        layer: request.layer,
    });
}

fn create_layer(
    request: On<CreateLayer>,
    mut registry: ResMut<LayerRegistry>,
    mut commands: Commands,
) {
    let layer = registry.insert(LayerDefinition::categorical(request.name.trim()));
    commands.trigger(LayerDefinitionChanged { layer });
    commands.trigger(LayerCreated { layer });
}

fn create_region(
    request: On<CreateRegion>,
    mut registry: ResMut<LayerRegistry>,
    mut commands: Commands,
) {
    let Some(layer) = registry.definitions.get_mut(&request.layer) else {
        commands.trigger(LayerEditFailed(LayerEditError::MissingLayer(request.layer)));
        return;
    };
    if layer.locked {
        commands.trigger(LayerEditFailed(LayerEditError::LockedLayer(request.layer)));
        return;
    }
    let region = layer.add_region(request.name.trim(), request.color_rgba);
    commands.trigger(LayerDefinitionChanged {
        layer: request.layer,
    });
    commands.trigger(RegionCreated {
        layer: request.layer,
        region,
    });
}

fn paint_region_request(
    request: On<PaintRegion>,
    registry: Res<LayerRegistry>,
    mut layers: ResMut<LayerStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let stamp = request.stamp;
    let Some(definition) = registry.definitions.get(&stamp.layer) else {
        commands.trigger(LayerEditFailed(LayerEditError::MissingLayer(stamp.layer)));
        return;
    };
    if definition.locked {
        commands.trigger(LayerEditFailed(LayerEditError::LockedLayer(stamp.layer)));
        return;
    }
    let tile = layers.ensure_region_tile(stamp.layer, stamp.tile, request.edge_cells);
    let patch = paint_region(tile, stamp);
    if patch.changes.is_empty() {
        return;
    }
    if let Some(history) = history.as_deref_mut() {
        history.record(RegionUndo(patch));
    }
    commands.trigger(RegionChanged {
        layer: stamp.layer,
        tile: stamp.tile,
        revision: tile.revision,
    });
}

impl LayerStore {
    pub fn ensure_region_tile(
        &mut self,
        layer: LayerId,
        tile: TileId,
        edge_cells: u16,
    ) -> &mut RegionTile {
        self.categorical_tiles
            .entry((layer, tile))
            .or_insert_with(|| RegionTile::empty(tile, edge_cells))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RegionBrushStamp {
    pub layer: LayerId,
    pub tile: TileId,
    pub region: RegionCode,
    pub center_uv: [f32; 2],
    pub radius_cells: f32,
    pub falloff: BrushFalloff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegionChange {
    pub index: u32,
    pub before: RegionCode,
    pub after: RegionCode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPatch {
    pub layer: LayerId,
    pub tile: TileId,
    pub changes: Vec<RegionChange>,
}

struct RegionUndo(RegionPatch);

impl UndoAction for RegionUndo {
    fn undo(&self, world: &mut World) -> Result<(), String> {
        apply_history_patch(world, &self.0, false)
    }

    fn redo(&self, world: &mut World) -> Result<(), String> {
        apply_history_patch(world, &self.0, true)
    }
}

fn apply_history_patch(
    world: &mut World,
    patch: &RegionPatch,
    forward: bool,
) -> Result<(), String> {
    let revision = {
        let mut layers = world
            .get_resource_mut::<LayerStore>()
            .ok_or_else(|| "layer plugin is not installed".to_string())?;
        let tile = layers
            .categorical_tiles
            .get_mut(&(patch.layer, patch.tile))
            .ok_or_else(|| "region tile needed by history is not resident".to_string())?;
        apply_region_patch(tile, patch, forward);
        tile.revision
    };
    world.trigger(RegionChanged {
        layer: patch.layer,
        tile: patch.tile,
        revision,
    });
    Ok(())
}

pub fn paint_region(tile: &mut RegionTile, stamp: RegionBrushStamp) -> RegionPatch {
    let edge = tile.edge_cells as usize;
    let center = [
        stamp.center_uv[0] * edge as f32,
        stamp.center_uv[1] * edge as f32,
    ];
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
            if distance > radius || stamp.falloff.weight(distance / radius) < 0.5 {
                continue;
            }
            let index = tile.index(x, y);
            let before = tile.cells[index];
            if before != stamp.region {
                tile.cells[index] = stamp.region;
                changes.push(RegionChange {
                    index: index as u32,
                    before,
                    after: stamp.region,
                });
            }
        }
    }

    if !changes.is_empty() {
        tile.revision = tile.revision.wrapping_add(1);
    }
    RegionPatch {
        layer: stamp.layer,
        tile: stamp.tile,
        changes,
    }
}

pub fn apply_region_patch(tile: &mut RegionTile, patch: &RegionPatch, forward: bool) {
    for change in &patch.changes {
        tile.cells[change.index as usize] = if forward { change.after } else { change.before };
    }
    if !patch.changes.is_empty() {
        tile.revision = tile.revision.wrapping_add(1);
    }
}
