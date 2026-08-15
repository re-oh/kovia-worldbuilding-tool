use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    coordinates::SurfacePosition,
    history::{EditHistory, UndoAction},
    layers::LayerId,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementKind {
    Hamlet,
    Village,
    Town,
    City,
    Capital,
    Port,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FeatureGeometry {
    Point(SurfacePosition),
    Path(Vec<SurfacePosition>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FeatureKind {
    Settlement {
        kind: SettlementKind,
        population: Option<u64>,
    },
    River {
        discharge_m3_s: Option<f32>,
        locked: bool,
    },
    Road,
    SeaRoute,
    Border,
    Landmark,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapFeature {
    pub id: FeatureId,
    pub name: String,
    pub kind: FeatureKind,
    pub geometry: FeatureGeometry,
    pub visible_from_level: u8,
    pub layer: Option<LayerId>,
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct FeatureStore {
    pub features: BTreeMap<FeatureId, MapFeature>,
}

#[derive(Debug, Clone, Event)]
pub struct PlaceSettlement {
    pub name: String,
    pub position: SurfacePosition,
    pub kind: SettlementKind,
    pub population: Option<u64>,
    pub layer: Option<LayerId>,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct FeatureChanged {
    pub feature: FeatureId,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct FeatureCreated {
    pub feature: FeatureId,
}

#[derive(Debug, Clone, Event)]
pub struct UpdateSettlement {
    pub feature: FeatureId,
    pub name: String,
    pub kind: SettlementKind,
    pub population: Option<u64>,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct DeleteFeature {
    pub feature: FeatureId,
}

#[derive(Debug, Clone, Event)]
pub struct FeatureEditFailed(pub String);

/// Owns semantic point and vector features. It has no terrain, editor, UI, or
/// renderer dependency.
pub struct FeaturePlugin;

impl Plugin for FeaturePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FeatureStore>()
            .add_observer(place_settlement)
            .add_observer(update_settlement)
            .add_observer(delete_feature);
    }
}

fn place_settlement(
    request: On<PlaceSettlement>,
    mut features: ResMut<FeatureStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let name = request.name.trim();
    if name.is_empty() {
        commands.trigger(FeatureEditFailed("settlement name cannot be empty".into()));
        return;
    }
    let feature = MapFeature {
        id: FeatureId::new(),
        name: name.to_string(),
        kind: FeatureKind::Settlement {
            kind: request.kind,
            population: request.population,
        },
        geometry: FeatureGeometry::Point(request.position),
        visible_from_level: match request.kind {
            SettlementKind::Capital => 0,
            SettlementKind::City | SettlementKind::Port => 2,
            SettlementKind::Town => 4,
            SettlementKind::Village | SettlementKind::Hamlet => 6,
        },
        layer: request.layer,
    };
    let id = features.insert(feature.clone());
    if let Some(history) = history.as_deref_mut() {
        history.record(FeatureInsertionUndo(feature));
    }
    commands.trigger(FeatureCreated { feature: id });
    commands.trigger(FeatureChanged { feature: id });
}

fn update_settlement(
    request: On<UpdateSettlement>,
    mut features: ResMut<FeatureStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let Some(feature) = features.features.get_mut(&request.feature) else {
        commands.trigger(FeatureEditFailed(
            "selected feature no longer exists".into(),
        ));
        return;
    };
    let before = feature.clone();
    let FeatureKind::Settlement { kind, population } = &mut feature.kind else {
        commands.trigger(FeatureEditFailed(
            "selected feature is not a settlement".into(),
        ));
        return;
    };
    let name = request.name.trim();
    if name.is_empty() {
        commands.trigger(FeatureEditFailed("settlement name cannot be empty".into()));
        return;
    }
    feature.name = name.to_string();
    *kind = request.kind;
    *population = request.population;
    let after = feature.clone();
    if after == before {
        return;
    }
    if let Some(history) = history.as_deref_mut() {
        history.record(FeatureReplacementUndo { before, after });
    }
    commands.trigger(FeatureChanged {
        feature: request.feature,
    });
}

fn delete_feature(
    request: On<DeleteFeature>,
    mut features: ResMut<FeatureStore>,
    mut history: Option<ResMut<EditHistory>>,
    mut commands: Commands,
) {
    let Some(feature) = features.features.remove(&request.feature) else {
        commands.trigger(FeatureEditFailed(
            "selected feature no longer exists".into(),
        ));
        return;
    };
    if let Some(history) = history.as_deref_mut() {
        history.record(FeatureRemovalUndo(feature));
    }
    commands.trigger(FeatureChanged {
        feature: request.feature,
    });
}

struct FeatureInsertionUndo(MapFeature);

impl UndoAction for FeatureInsertionUndo {
    fn undo(&self, world: &mut World) -> Result<(), String> {
        {
            let mut features = world
                .get_resource_mut::<FeatureStore>()
                .ok_or_else(|| "feature plugin is not installed".to_string())?;
            features
                .features
                .remove(&self.0.id)
                .ok_or_else(|| "feature needed by history is no longer present".to_string())?;
        }
        world.trigger(FeatureChanged { feature: self.0.id });
        Ok(())
    }

    fn redo(&self, world: &mut World) -> Result<(), String> {
        {
            let mut features = world
                .get_resource_mut::<FeatureStore>()
                .ok_or_else(|| "feature plugin is not installed".to_string())?;
            features.insert(self.0.clone());
        }
        world.trigger(FeatureChanged { feature: self.0.id });
        Ok(())
    }
}

struct FeatureRemovalUndo(MapFeature);

impl UndoAction for FeatureRemovalUndo {
    fn undo(&self, world: &mut World) -> Result<(), String> {
        replace_feature(world, self.0.id, Some(self.0.clone()))
    }

    fn redo(&self, world: &mut World) -> Result<(), String> {
        replace_feature(world, self.0.id, None)
    }
}

struct FeatureReplacementUndo {
    before: MapFeature,
    after: MapFeature,
}

impl UndoAction for FeatureReplacementUndo {
    fn undo(&self, world: &mut World) -> Result<(), String> {
        replace_feature(world, self.before.id, Some(self.before.clone()))
    }

    fn redo(&self, world: &mut World) -> Result<(), String> {
        replace_feature(world, self.after.id, Some(self.after.clone()))
    }
}

fn replace_feature(
    world: &mut World,
    id: FeatureId,
    replacement: Option<MapFeature>,
) -> Result<(), String> {
    let mut features = world
        .get_resource_mut::<FeatureStore>()
        .ok_or_else(|| "feature plugin is not installed".to_string())?;
    match replacement {
        Some(feature) => {
            features.features.insert(id, feature);
        }
        None => {
            features.features.remove(&id);
        }
    }
    world.trigger(FeatureChanged { feature: id });
    Ok(())
}

impl FeatureStore {
    pub fn insert(&mut self, feature: MapFeature) -> FeatureId {
        let id = feature.id;
        self.features.insert(id, feature);
        id
    }

    pub fn nearest_point(
        &self,
        face: crate::coordinates::CubeFace,
        uv: [f64; 2],
        maximum_distance: f64,
    ) -> Option<FeatureId> {
        self.features
            .values()
            .filter_map(|feature| {
                let FeatureGeometry::Point(position) = feature.geometry else {
                    return None;
                };
                if position.face != face {
                    return None;
                }
                let distance = ((position.u - uv[0]).powi(2) + (position.v - uv[1]).powi(2)).sqrt();
                (distance <= maximum_distance).then_some((distance, feature.id))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        coordinates::CubeFace,
        history::{HistoryPlugin, Undo},
    };

    fn settlement(name: &str, u: f64, v: f64) -> PlaceSettlement {
        PlaceSettlement {
            name: name.into(),
            position: SurfacePosition {
                face: CubeFace::PositiveY,
                u,
                v,
                altitude_m: 100.0,
            },
            kind: SettlementKind::Town,
            population: Some(2_000),
            layer: None,
        }
    }

    #[test]
    fn settlement_create_update_delete_and_undo_use_domain_events() {
        let mut app = App::new();
        app.add_plugins((HistoryPlugin, FeaturePlugin));
        app.world_mut().trigger(settlement("Talar", 0.1, -0.2));
        let id = *app
            .world()
            .resource::<FeatureStore>()
            .features
            .keys()
            .next()
            .unwrap();

        app.world_mut().trigger(UpdateSettlement {
            feature: id,
            name: "New Talar".into(),
            kind: SettlementKind::Capital,
            population: Some(12_000),
        });
        assert_eq!(
            app.world().resource::<FeatureStore>().features[&id].name,
            "New Talar"
        );

        app.world_mut().trigger(DeleteFeature { feature: id });
        assert!(app.world().resource::<FeatureStore>().features.is_empty());
        app.world_mut().trigger(Undo);
        app.update();
        assert_eq!(
            app.world().resource::<FeatureStore>().features[&id].name,
            "New Talar"
        );
    }

    #[test]
    fn nearest_point_respects_face_and_radius() {
        let mut store = FeatureStore::default();
        let feature = MapFeature {
            id: FeatureId::new(),
            name: "Hyrel".into(),
            kind: FeatureKind::Settlement {
                kind: SettlementKind::Port,
                population: None,
            },
            geometry: FeatureGeometry::Point(SurfacePosition {
                face: CubeFace::PositiveY,
                u: 0.4,
                v: 0.2,
                altitude_m: 0.0,
            }),
            visible_from_level: 0,
            layer: None,
        };
        let id = store.insert(feature);
        assert_eq!(
            store.nearest_point(CubeFace::PositiveY, [0.41, 0.2], 0.02),
            Some(id)
        );
        assert_eq!(
            store.nearest_point(CubeFace::NegativeY, [0.41, 0.2], 0.02),
            None
        );
    }
}
