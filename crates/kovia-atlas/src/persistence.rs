use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    change_tracking::DirtyTracker,
    coordinates::PlanetSpec,
    editor::{ActiveEditTile, LayerSelection, SelectedFeature},
    features::{FeatureChanged, FeatureGeometry, FeatureStore, MapFeature},
    history::EditHistory,
    hydrology::HydrologyCache,
    layers::{
        LayerDefinition, LayerDefinitionChanged, LayerRegistry, LayerStore, RegionChanged,
        RegionTile,
    },
    terrain::{HeightTile, TerrainChanged, TerrainStore},
};

const PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Resource)]
pub struct ProjectPath(pub String);

impl Default for ProjectPath {
    fn default() -> Self {
        Self("kovia-atlas.kovia.json".into())
    }
}

#[derive(Debug, Clone, Default, Resource)]
pub struct ProjectSession {
    pub dirty: bool,
    pub last_saved_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct SaveProject;

#[derive(Debug, Clone, Copy, Event)]
pub struct LoadProject;

#[derive(Debug, Clone, Event)]
pub struct ProjectSaved {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Event)]
pub struct ProjectLoaded {
    pub path: PathBuf,
}

#[derive(Debug, Clone, Event)]
pub struct ProjectIoFailed {
    pub operation: &'static str,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectSnapshot {
    schema_version: u32,
    planet: PlanetSpec,
    terrain_tiles: Vec<HeightTile>,
    layers: Vec<LayerDefinition>,
    categorical_tiles: Vec<ProjectRegionTile>,
    features: Vec<MapFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProjectRegionTile {
    layer: crate::layers::LayerId,
    tile: RegionTile,
}

pub struct ProjectPersistencePlugin;

impl Plugin for ProjectPersistencePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ProjectPath>()
            .init_resource::<ProjectSession>()
            .add_observer(save_project)
            .add_observer(load_project)
            .add_observer(mark_dirty_from_terrain)
            .add_observer(mark_dirty_from_regions)
            .add_observer(mark_dirty_from_layer_definition)
            .add_observer(mark_dirty_from_features);
    }
}

#[derive(SystemParam)]
struct SaveProjectParams<'w> {
    path: Res<'w, ProjectPath>,
    planet: Res<'w, PlanetSpec>,
    terrain: Res<'w, TerrainStore>,
    layer_registry: Res<'w, LayerRegistry>,
    layer_store: Res<'w, LayerStore>,
    features: Res<'w, FeatureStore>,
    session: ResMut<'w, ProjectSession>,
}

fn save_project(_request: On<SaveProject>, mut params: SaveProjectParams, mut commands: Commands) {
    let target = PathBuf::from(params.path.0.trim());
    let snapshot = ProjectSnapshot::from_resources(
        *params.planet,
        &params.terrain,
        &params.layer_registry,
        &params.layer_store,
        &params.features,
    );
    match write_snapshot_atomic(&target, &snapshot) {
        Ok(()) => {
            params.session.dirty = false;
            params.session.last_saved_path = Some(target.clone());
            commands.trigger(ProjectSaved { path: target });
        }
        Err(error) => commands.trigger(ProjectIoFailed {
            operation: "save",
            error,
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn load_project(
    _request: On<LoadProject>,
    path: Res<ProjectPath>,
    mut planet: ResMut<PlanetSpec>,
    mut terrain: ResMut<TerrainStore>,
    mut layer_registry: ResMut<LayerRegistry>,
    mut layer_store: ResMut<LayerStore>,
    mut features: ResMut<FeatureStore>,
    mut history: ResMut<EditHistory>,
    mut dirty: ResMut<DirtyTracker>,
    mut hydrology: ResMut<HydrologyCache>,
    mut selection: ResMut<LayerSelection>,
    mut selected_feature: ResMut<SelectedFeature>,
    mut active_tile: ResMut<ActiveEditTile>,
    mut session: ResMut<ProjectSession>,
    mut commands: Commands,
) {
    let target = PathBuf::from(path.0.trim());
    let result = fs::read(&target)
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<ProjectSnapshot>(&bytes).map_err(|e| e.to_string())
        })
        .and_then(|snapshot| {
            validate_snapshot(&snapshot)?;
            Ok(snapshot)
        });
    let snapshot = match result {
        Ok(snapshot) => snapshot,
        Err(error) => {
            commands.trigger(ProjectIoFailed {
                operation: "load",
                error,
            });
            return;
        }
    };

    let (loaded_planet, loaded_terrain, loaded_registry, loaded_layers, loaded_features) =
        snapshot.into_resources();
    *planet = loaded_planet;
    *terrain = loaded_terrain;
    *layer_registry = loaded_registry;
    *layer_store = loaded_layers;
    *features = loaded_features;
    history.clear();
    *dirty = DirtyTracker::default();
    *hydrology = HydrologyCache::default();
    selection.layer = layer_registry.ordering.first().copied();
    selection.region = selection
        .layer
        .and_then(|id| layer_registry.definitions.get(&id))
        .and_then(|layer| layer.regions.keys().next().copied());
    selected_feature.0 = None;
    if !terrain.tiles.contains_key(&active_tile.0)
        && let Some(tile) = terrain.tiles.keys().next().copied()
    {
        active_tile.0 = tile;
    }
    session.dirty = false;
    session.last_saved_path = Some(target.clone());
    commands.trigger(ProjectLoaded { path: target });
}

fn write_snapshot_atomic(path: &Path, snapshot: &ProjectSnapshot) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("project path is empty".into());
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file_name = path
        .file_name()
        .ok_or_else(|| "project path has no file name".to_string())?
        .to_string_lossy();
    let temporary = path.with_file_name(format!(".{file_name}.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(|error| error.to_string())?;
    let write_result = (|| {
        let mut file = File::create(&temporary).map_err(|error| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

impl ProjectSnapshot {
    fn from_resources(
        planet: PlanetSpec,
        terrain: &TerrainStore,
        registry: &LayerRegistry,
        layer_store: &LayerStore,
        features: &FeatureStore,
    ) -> Self {
        Self {
            schema_version: PROJECT_SCHEMA_VERSION,
            planet,
            terrain_tiles: terrain.tiles.values().cloned().collect(),
            layers: registry
                .ordering
                .iter()
                .filter_map(|id| registry.definitions.get(id).cloned())
                .collect(),
            categorical_tiles: layer_store
                .categorical_tiles
                .iter()
                .map(|(&(layer, _), tile)| ProjectRegionTile {
                    layer,
                    tile: tile.clone(),
                })
                .collect(),
            features: features.features.values().cloned().collect(),
        }
    }

    fn into_resources(
        self,
    ) -> (
        PlanetSpec,
        TerrainStore,
        LayerRegistry,
        LayerStore,
        FeatureStore,
    ) {
        let terrain = TerrainStore {
            tiles: self
                .terrain_tiles
                .into_iter()
                .map(|tile| (tile.id, tile))
                .collect(),
        };
        let mut registry = LayerRegistry::default();
        for layer in self.layers {
            registry.insert(layer);
        }
        let layers = LayerStore {
            categorical_tiles: self
                .categorical_tiles
                .into_iter()
                .map(|record| ((record.layer, record.tile.tile), record.tile))
                .collect(),
        };
        let features = FeatureStore {
            features: self
                .features
                .into_iter()
                .map(|feature| (feature.id, feature))
                .collect(),
        };
        (self.planet, terrain, registry, layers, features)
    }
}

fn validate_snapshot(snapshot: &ProjectSnapshot) -> Result<(), String> {
    if snapshot.schema_version != PROJECT_SCHEMA_VERSION {
        return Err(format!(
            "unsupported project schema {}; expected {}",
            snapshot.schema_version, PROJECT_SCHEMA_VERSION
        ));
    }
    if !snapshot.planet.radius_m.is_finite() || snapshot.planet.radius_m <= 0.0 {
        return Err("planet radius must be a positive finite number".into());
    }
    let mut terrain_ids = std::collections::BTreeSet::new();
    for tile in &snapshot.terrain_tiles {
        if !terrain_ids.insert(tile.id) || tile.edge_samples < 2 {
            return Err("terrain tile metadata is inconsistent".into());
        }
        let expected = tile.edge_samples as usize * tile.edge_samples as usize;
        if tile.samples().len() != expected
            || tile.samples().iter().any(|height| !height.is_finite())
        {
            return Err(format!("terrain tile {:?} has invalid samples", tile.id));
        }
    }
    let mut layer_ids = std::collections::BTreeSet::new();
    for layer in &snapshot.layers {
        if !layer_ids.insert(layer.id) {
            return Err(format!("duplicate layer {:?}", layer.id));
        }
    }
    let mut region_tile_ids = std::collections::BTreeSet::new();
    for record in &snapshot.categorical_tiles {
        if !layer_ids.contains(&record.layer)
            || !region_tile_ids.insert((record.layer, record.tile.tile))
            || record.tile.cell_count()
                != record.tile.edge_cells as usize * record.tile.edge_cells as usize
        {
            return Err("categorical tile metadata is inconsistent".into());
        }
    }
    let mut feature_ids = std::collections::BTreeSet::new();
    for feature in &snapshot.features {
        if !feature_ids.insert(feature.id) {
            return Err(format!("duplicate feature {:?}", feature.id));
        }
        if feature
            .layer
            .is_some_and(|layer| !layer_ids.contains(&layer))
        {
            return Err(format!(
                "feature {:?} references a missing layer",
                feature.id
            ));
        }
        let positions: &[crate::coordinates::SurfacePosition] = match &feature.geometry {
            FeatureGeometry::Point(position) => std::slice::from_ref(position),
            FeatureGeometry::Path(positions) => positions,
        };
        if positions.iter().any(|position| {
            !position.u.is_finite()
                || !position.v.is_finite()
                || !position.altitude_m.is_finite()
                || !(-1.0..=1.0).contains(&position.u)
                || !(-1.0..=1.0).contains(&position.v)
        }) {
            return Err(format!("feature {:?} has an invalid position", feature.id));
        }
    }
    Ok(())
}

fn mark_dirty_from_terrain(_event: On<TerrainChanged>, mut session: ResMut<ProjectSession>) {
    session.dirty = true;
}

fn mark_dirty_from_regions(_event: On<RegionChanged>, mut session: ResMut<ProjectSession>) {
    session.dirty = true;
}

fn mark_dirty_from_layer_definition(
    _event: On<LayerDefinitionChanged>,
    mut session: ResMut<ProjectSession>,
) {
    session.dirty = true;
}

fn mark_dirty_from_features(_event: On<FeatureChanged>, mut session: ResMut<ProjectSession>) {
    session.dirty = true;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        change_tracking::ChangeTrackingPlugin,
        coordinates::PlanetPlugin,
        coordinates::TileId,
        editor::EditorStatePlugin,
        features::FeaturePlugin,
        history::HistoryPlugin,
        hydrology::HydrologyPlugin,
        layers::LayerDefinition,
        layers::LayerPlugin,
        terrain::TerrainPlugin,
        terrain::{DEFAULT_TILE_CELLS, HeightTile},
    };

    fn snapshot() -> ProjectSnapshot {
        let mut terrain = TerrainStore::default();
        terrain.tiles.insert(
            TileId::ROOT,
            HeightTile::flat(TileId::ROOT, DEFAULT_TILE_CELLS + 1, 42.0),
        );
        let mut layer_registry = LayerRegistry::default();
        layer_registry.insert(LayerDefinition::categorical("Historical polities"));
        ProjectSnapshot::from_resources(
            PlanetSpec::default(),
            &terrain,
            &layer_registry,
            &LayerStore::default(),
            &FeatureStore::default(),
        )
    }

    #[test]
    fn snapshot_round_trips_without_losing_authored_data() {
        let source = snapshot();
        let bytes = serde_json::to_vec(&source).unwrap();
        let restored: ProjectSnapshot = serde_json::from_slice(&bytes).unwrap();
        validate_snapshot(&restored).unwrap();
        let (_, terrain, registry, _, _) = restored.into_resources();
        assert_eq!(terrain.tiles[&TileId::ROOT].get(0, 0), 42.0);
        assert_eq!(registry.ordering.len(), 1);
    }

    #[test]
    fn atomic_write_creates_a_loadable_project() {
        let directory = std::env::temp_dir().join(format!(
            "kovia-atlas-persistence-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let path = directory.join("world.kovia.json");
        write_snapshot_atomic(&path, &snapshot()).unwrap();
        let restored: ProjectSnapshot = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        validate_snapshot(&restored).unwrap();
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn plugin_save_then_load_restores_live_authoritative_state() {
        let directory = std::env::temp_dir().join(format!(
            "kovia-atlas-live-load-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let path = directory.join("world.kovia.json");
        let mut app = App::new();
        app.add_plugins(PlanetPlugin)
            .add_plugins(TerrainPlugin)
            .add_plugins(LayerPlugin)
            .add_plugins(FeaturePlugin)
            .add_plugins(HistoryPlugin)
            .add_plugins(ChangeTrackingPlugin)
            .add_plugins(HydrologyPlugin)
            .add_plugins(EditorStatePlugin)
            .add_plugins(ProjectPersistencePlugin);
        app.world_mut().resource_mut::<ProjectPath>().0 = path.display().to_string();
        app.world_mut()
            .resource_mut::<TerrainStore>()
            .tiles
            .insert(TileId::ROOT, HeightTile::flat(TileId::ROOT, 3, 77.0));
        app.world_mut().trigger(SaveProject);

        app.world_mut()
            .resource_mut::<TerrainStore>()
            .tiles
            .get_mut(&TileId::ROOT)
            .unwrap()
            .set(0, 0, -500.0);
        app.world_mut().trigger(LoadProject);

        assert_eq!(
            app.world().resource::<TerrainStore>().tiles[&TileId::ROOT].get(0, 0),
            77.0
        );
        assert!(!app.world().resource::<ProjectSession>().dirty);
        fs::remove_dir_all(directory).unwrap();
    }
}
