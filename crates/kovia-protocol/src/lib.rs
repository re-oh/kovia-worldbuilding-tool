//! Framework-neutral values crossing the Iced/Atlas boundary.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct FeatureId(pub Uuid);

impl FeatureId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FeatureId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct LayerId(pub Uuid);

impl LayerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for LayerId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RegionCode(pub u16);

impl RegionCode {
    pub const UNASSIGNED: Self = Self(0);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionRef {
    pub layer: LayerId,
    pub code: RegionCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PointerButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum MapInput {
    PointerMoved {
        physical_position: [f32; 2],
    },
    PointerDown {
        physical_position: [f32; 2],
        button: PointerButton,
    },
    PointerUp {
        physical_position: [f32; 2],
        button: PointerButton,
    },
    Scroll {
        physical_delta: [f32; 2],
    },
    Resize {
        physical_size: [u32; 2],
        scale_factor: f64,
    },
    FocusChanged(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapTool {
    Navigate,
    Sculpt,
    Regions,
    Settlement,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BrushSettings {
    pub radius_cells: f32,
    pub strength: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius_cells: 8.0,
            strength: 35.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapCommand {
    SetTool(MapTool),
    SetBrush(BrushSettings),
    Undo,
    Redo,
    SaveProject,
    LoadProject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureSummary {
    pub id: FeatureId,
    pub name: String,
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerSummary {
    pub id: LayerId,
    pub name: String,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CameraSummary {
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for CameraSummary {
    fn default() -> Self {
        Self {
            distance: 22.0,
            yaw: 0.72,
            pitch: -0.68,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapSnapshot {
    pub revision: u64,
    pub selected_feature: Option<FeatureSummary>,
    pub selected_layer: Option<LayerSummary>,
    pub available_layers: Vec<LayerSummary>,
    pub active_tool: MapTool,
    pub brush: BrushSettings,
    pub camera: CameraSummary,
    pub project_dirty: bool,
    pub undo_available: bool,
    pub redo_available: bool,
    pub status: String,
}

impl Default for MapSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            selected_feature: None,
            selected_layer: None,
            available_layers: Vec::new(),
            active_tool: MapTool::Sculpt,
            brush: BrushSettings::default(),
            camera: CameraSummary::default(),
            project_dirty: false,
            undo_available: false,
            redo_available: false,
            status: "Atlas ready".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MapEvent {
    SelectionChanged(Option<FeatureId>),
    CameraChanged(CameraSummary),
    CommandAccepted { revision: u64 },
    CommandRejected { error: String },
    ProjectChanged { dirty: bool },
    ProjectSaved { path: String },
    ProjectLoaded { path: String },
    ProjectIoFailed { operation: String, error: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_ids_keep_the_existing_tuple_json_shape() {
        let id = FeatureId(Uuid::nil());
        assert_eq!(
            serde_json::to_string(&id).unwrap(),
            r#""00000000-0000-0000-0000-000000000000""#
        );
        assert_eq!(serde_json::to_string(&RegionCode(7)).unwrap(), "7");
    }

    #[test]
    fn region_references_are_layer_scoped() {
        let a = RegionRef {
            layer: LayerId(Uuid::nil()),
            code: RegionCode(1),
        };
        let b = RegionRef {
            layer: LayerId(Uuid::from_u128(1)),
            code: RegionCode(1),
        };
        assert_ne!(a, b);
    }
}
