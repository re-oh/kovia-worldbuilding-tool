use bevy::prelude::*;

use crate::{
    brush::BrushFalloff,
    coordinates::TileId,
    features::{FeatureId, SettlementKind},
    layers::{LayerId, RegionCode},
    terrain::SculptMode,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Resource)]
pub enum EditorTool {
    Navigate,
    #[default]
    Sculpt,
    Regions,
    City,
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct TerrainBrushSettings {
    pub mode: SculptMode,
    pub falloff: BrushFalloff,
    pub radius_cells: f32,
    pub strength: f32,
}

impl Default for TerrainBrushSettings {
    fn default() -> Self {
        Self {
            mode: SculptMode::Raise,
            falloff: BrushFalloff::Smooth,
            radius_cells: 8.0,
            strength: 35.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct LayerSelection {
    pub layer: Option<LayerId>,
    pub region: Option<RegionCode>,
}

#[derive(Debug, Clone, Resource)]
pub struct SettlementDraft {
    pub name: String,
    pub kind: SettlementKind,
}

#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct SelectedFeature(pub Option<FeatureId>);

impl Default for SettlementDraft {
    fn default() -> Self {
        Self {
            name: "New city".into(),
            kind: SettlementKind::City,
        }
    }
}

#[derive(Debug, Clone, Copy, Resource)]
pub struct ActiveEditTile(pub TileId);

impl Default for ActiveEditTile {
    fn default() -> Self {
        Self(TileId::ROOT)
    }
}

#[derive(Debug, Default, Resource)]
pub struct StrokeState {
    pub last_stamp_uv: Option<[f32; 2]>,
}

#[derive(Debug, Resource)]
pub struct EditorDraftNames {
    pub layer: String,
    pub region: String,
}

impl Default for EditorDraftNames {
    fn default() -> Self {
        Self {
            layer: "New layer".into(),
            region: "New region".into(),
        }
    }
}

#[derive(Debug, Resource)]
pub struct StatusLine(pub String);

impl Default for StatusLine {
    fn default() -> Self {
        Self("Ready".into())
    }
}

/// Owns transient editor intent only. It does not install UI, input, domain
/// data, rendering, or simulation systems.
pub struct EditorStatePlugin;

impl Plugin for EditorStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorTool>()
            .init_resource::<TerrainBrushSettings>()
            .init_resource::<LayerSelection>()
            .init_resource::<SettlementDraft>()
            .init_resource::<SelectedFeature>()
            .init_resource::<ActiveEditTile>()
            .init_resource::<StrokeState>()
            .init_resource::<EditorDraftNames>()
            .init_resource::<StatusLine>();
    }
}
