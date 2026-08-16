//! Shared-device Atlas host used by the combined Kovia workbench.

use std::sync::Arc;

use bevy::camera::{Camera, Camera3d, ManualTextureViewHandle, RenderTarget};
use bevy::prelude::*;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use bevy::render::renderer::{
    RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue, WgpuWrapper,
};
use bevy::render::settings::RenderCreation;
use bevy::render::texture::{ManualTextureView, ManualTextureViews};
use bevy::render::{RenderPlugin, render_resource};
use bevy::window::{ExitCondition, WindowPlugin};
use kovia_protocol::{
    BrushSettings, CameraSummary, FeatureSummary, LayerSummary, MapCommand, MapEvent, MapInput,
    MapSnapshot, MapTool, PointerButton,
};

use crate::camera::{AtlasCamera, OrbitCamera};
use crate::change_tracking::ChangeTrackingPlugin;
use crate::coordinates::{PlanetPlugin, SurfacePosition, TileId};
use crate::editor::{
    ActiveEditTile, EditorStatePlugin, EditorTool, LayerSelection, SelectedFeature,
    SettlementDraft, StatusLine, StrokeState, TerrainBrushSettings,
};
use crate::erosion::ErosionPlugin;
use crate::feature_render::FeatureRenderPlugin;
use crate::features::{
    FeatureCreated, FeatureEditFailed, FeatureKind, FeaturePlugin, FeatureStore, PlaceSettlement,
};
use crate::history::{EditHistory, HistoryOutcome, HistoryPlugin, Redo, Undo};
use crate::hydrology::HydrologyPlugin;
use crate::interaction::pick_terrain_surface;
use crate::layers::{LayerPlugin, LayerRegistry, PaintRegion, RegionBrushStamp};
use crate::overlay_render::HydrologyOverlayRenderPlugin;
use crate::persistence::{
    LoadProject, ProjectIoFailed, ProjectLoaded, ProjectPath, ProjectPersistencePlugin,
    ProjectSaved, ProjectSession, SaveProject,
};
use crate::starter_world::StarterWorldPlugin;
use crate::streaming::TileResidencyPlugin;
use crate::terrain::{
    HeightBrushStamp, SculptTerrain, TerrainChanged, TerrainPlugin, TerrainStore,
};
use crate::terrain_render::TerrainRenderPlugin;

pub const VIEW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const VIEW_HANDLE: ManualTextureViewHandle = ManualTextureViewHandle(0x4b4f_5649);

#[derive(Resource, Default)]
struct BridgeState {
    inputs: Vec<MapInput>,
    commands: Vec<MapCommand>,
    events: Vec<MapEvent>,
    snapshot: MapSnapshot,
    revision: u64,
}

#[derive(Resource)]
struct ViewportState {
    pointer: Option<Vec2>,
    pointer_delta: Vec2,
    scroll: Vec2,
    physical_size: UVec2,
    scale_factor: f64,
    left_down: bool,
    right_down: bool,
    left_just_pressed: bool,
    focused: bool,
}

impl ViewportState {
    fn new(size: [u32; 2]) -> Self {
        Self {
            pointer: None,
            pointer_delta: Vec2::ZERO,
            scroll: Vec2::ZERO,
            physical_size: UVec2::new(size[0], size[1]),
            scale_factor: 1.0,
            left_down: false,
            right_down: false,
            left_just_pressed: false,
            focused: true,
        }
    }
}

/// A manually advanced Bevy application using GPU handles owned by the shell.
pub struct AtlasEngine {
    app: App,
}

impl AtlasEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new_demo(
        instance: &wgpu::Instance,
        adapter: &wgpu::Adapter,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_view: wgpu::TextureView,
        physical_size: [u32; 2],
    ) -> Self {
        let render_creation = RenderCreation::manual(
            RenderDevice::from(device.clone()),
            RenderQueue(Arc::new(WgpuWrapper::new(queue.clone()))),
            RenderAdapterInfo(WgpuWrapper::new(adapter.get_info())),
            RenderAdapter(Arc::new(WgpuWrapper::new(adapter.clone()))),
            RenderInstance(Arc::new(WgpuWrapper::new(instance.clone()))),
        );

        let mut app = App::new();
        app.insert_resource(ClearColor(Color::srgb(0.018, 0.022, 0.028)))
            .insert_resource(ViewportState::new(physical_size))
            .insert_resource(BridgeState::default())
            .add_plugins(
                DefaultPlugins
                    .build()
                    .disable::<PipelinedRenderingPlugin>()
                    .set(WindowPlugin {
                        primary_window: None,
                        exit_condition: ExitCondition::DontExit,
                        close_when_requested: false,
                        ..default()
                    })
                    .set(RenderPlugin {
                        render_creation,
                        synchronous_pipeline_compilation: true,
                        ..default()
                    }),
            )
            .add_plugins(PlanetPlugin)
            .add_plugins(TerrainPlugin)
            .add_plugins(LayerPlugin)
            .add_plugins(FeaturePlugin)
            .add_plugins(HistoryPlugin)
            .add_plugins(ChangeTrackingPlugin)
            .add_plugins(ProjectPersistencePlugin)
            .add_plugins(HydrologyPlugin)
            .add_plugins(ErosionPlugin)
            .add_plugins(TileResidencyPlugin)
            .add_plugins(EditorStatePlugin)
            .add_plugins(TerrainRenderPlugin)
            .add_plugins(FeatureRenderPlugin)
            .add_plugins(HydrologyOverlayRenderPlugin)
            .add_plugins(StarterWorldPlugin)
            .add_systems(Startup, spawn_hybrid_scene)
            .add_systems(
                Update,
                (
                    ingest_input,
                    process_commands,
                    orbit_camera,
                    interact_with_map,
                    refresh_snapshot,
                    finish_input_frame,
                )
                    .chain(),
            )
            .add_observer(on_feature_created)
            .add_observer(on_feature_failed)
            .add_observer(on_terrain_changed)
            .add_observer(on_history_outcome)
            .add_observer(on_project_saved)
            .add_observer(on_project_loaded)
            .add_observer(on_project_failed);

        // Manual applications have no runner to finish plugin initialization.
        app.finish();
        app.cleanup();
        insert_manual_view(&mut app, texture_view, physical_size);

        // The starter terrain is an explicit, visibly labelled demo fixture.
        app.world_mut().resource_mut::<ProjectPath>().0 = "kovia-demo.kovia.json".into();
        app.update();
        seed_demo_settlement(&mut app);
        app.update();
        app.world_mut().resource_mut::<ProjectSession>().dirty = false;

        Self { app }
    }

    pub fn replace_target(&mut self, texture_view: wgpu::TextureView, physical_size: [u32; 2]) {
        insert_manual_view(&mut self.app, texture_view, physical_size);
        self.send_input(MapInput::Resize {
            physical_size,
            scale_factor: self.app.world().resource::<ViewportState>().scale_factor,
        });
    }

    pub fn send_input(&mut self, input: MapInput) {
        self.app
            .world_mut()
            .resource_mut::<BridgeState>()
            .inputs
            .push(input);
    }

    pub fn send_command(&mut self, command: MapCommand) {
        self.app
            .world_mut()
            .resource_mut::<BridgeState>()
            .commands
            .push(command);
    }

    pub fn update(&mut self) -> Vec<MapEvent> {
        self.app.update();
        std::mem::take(&mut self.app.world_mut().resource_mut::<BridgeState>().events)
    }

    pub fn snapshot(&self) -> MapSnapshot {
        self.app.world().resource::<BridgeState>().snapshot.clone()
    }
}

fn insert_manual_view(app: &mut App, texture_view: wgpu::TextureView, size: [u32; 2]) {
    app.world_mut().resource_mut::<ManualTextureViews>().insert(
        VIEW_HANDLE,
        ManualTextureView {
            texture_view: render_resource::TextureView::from(texture_view),
            size: UVec2::new(size[0], size[1]),
            view_format: VIEW_FORMAT,
        },
    );
}

fn spawn_hybrid_scene(mut commands: Commands) {
    commands.spawn((
        Name::new("Kovia Atlas sun"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-8.0, 14.0, 7.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        Name::new("Kovia Atlas offscreen camera"),
        Camera3d::default(),
        Camera {
            clear_color: bevy::camera::ClearColorConfig::Custom(Color::srgb(0.018, 0.022, 0.028)),
            ..default()
        },
        RenderTarget::TextureView(VIEW_HANDLE),
        AtlasCamera,
        OrbitCamera::default(),
        Transform::default(),
    ));
}

fn seed_demo_settlement(app: &mut App) {
    let tile = TileId::ROOT;
    let altitude_m = app
        .world()
        .resource::<TerrainStore>()
        .tiles
        .get(&tile)
        .map_or(0.0, |height| height.sample_bilinear([0.48, 0.52]));
    let layer = app.world().resource::<LayerSelection>().layer;
    app.world_mut().trigger(PlaceSettlement {
        name: "Demo settlement".into(),
        position: SurfacePosition {
            face: tile.face,
            u: -0.04,
            v: 0.04,
            altitude_m,
        },
        kind: crate::features::SettlementKind::Port,
        population: Some(12_000),
        layer,
    });
    app.world_mut().resource_mut::<ProjectSession>().dirty = false;
    app.world_mut().resource_mut::<StatusLine>().0 =
        "Demo fixture — no project data or canon has been promoted".into();
}

fn ingest_input(mut bridge: ResMut<BridgeState>, mut viewport: ResMut<ViewportState>) {
    for input in std::mem::take(&mut bridge.inputs) {
        match input {
            MapInput::PointerMoved { physical_position } => {
                let next = Vec2::from_array(physical_position);
                if let Some(previous) = viewport.pointer {
                    viewport.pointer_delta += next - previous;
                }
                viewport.pointer = Some(next);
            }
            MapInput::PointerDown {
                physical_position,
                button,
            } => {
                viewport.pointer = Some(Vec2::from_array(physical_position));
                match button {
                    PointerButton::Left => {
                        viewport.left_just_pressed = !viewport.left_down;
                        viewport.left_down = true;
                    }
                    PointerButton::Right => viewport.right_down = true,
                    PointerButton::Middle => {}
                }
            }
            MapInput::PointerUp {
                physical_position,
                button,
            } => {
                viewport.pointer = Some(Vec2::from_array(physical_position));
                match button {
                    PointerButton::Left => viewport.left_down = false,
                    PointerButton::Right => viewport.right_down = false,
                    PointerButton::Middle => {}
                }
            }
            MapInput::Scroll { physical_delta } => {
                viewport.scroll += Vec2::from_array(physical_delta);
            }
            MapInput::Resize {
                physical_size,
                scale_factor,
            } => {
                viewport.physical_size = UVec2::from_array(physical_size);
                viewport.scale_factor = scale_factor;
            }
            MapInput::FocusChanged(focused) => {
                viewport.focused = focused;
                if !focused {
                    viewport.left_down = false;
                    viewport.right_down = false;
                    viewport.pointer = None;
                }
            }
        }
    }
}

fn process_commands(
    mut bridge: ResMut<BridgeState>,
    mut tool: ResMut<EditorTool>,
    mut brush: ResMut<TerrainBrushSettings>,
    mut status: ResMut<StatusLine>,
    mut commands: Commands,
) {
    for command in std::mem::take(&mut bridge.commands) {
        let result = match command {
            MapCommand::SetTool(next) => {
                *tool = match next {
                    MapTool::Navigate => EditorTool::Navigate,
                    MapTool::Sculpt => EditorTool::Sculpt,
                    MapTool::Regions => EditorTool::Regions,
                    MapTool::Settlement => EditorTool::City,
                };
                Ok(())
            }
            MapCommand::SetBrush(next) if valid_brush(next) => {
                brush.radius_cells = next.radius_cells;
                brush.strength = next.strength;
                Ok(())
            }
            MapCommand::SetBrush(_) => Err("Brush radius and strength must be finite and positive"),
            MapCommand::Undo => {
                commands.trigger(Undo);
                Ok(())
            }
            MapCommand::Redo => {
                commands.trigger(Redo);
                Ok(())
            }
            MapCommand::SaveProject => {
                commands.trigger(SaveProject);
                Ok(())
            }
            MapCommand::LoadProject => {
                commands.trigger(LoadProject);
                Ok(())
            }
        };
        match result {
            Ok(()) => {
                bridge.revision = bridge.revision.wrapping_add(1);
                let revision = bridge.revision;
                bridge.events.push(MapEvent::CommandAccepted { revision });
            }
            Err(error) => {
                status.0 = error.into();
                bridge.events.push(MapEvent::CommandRejected {
                    error: error.into(),
                });
            }
        }
    }
}

fn valid_brush(brush: BrushSettings) -> bool {
    brush.radius_cells.is_finite()
        && brush.radius_cells > 0.0
        && brush.strength.is_finite()
        && brush.strength > 0.0
}

fn orbit_camera(
    viewport: Res<ViewportState>,
    camera: Single<(&mut Transform, &mut OrbitCamera), With<AtlasCamera>>,
) {
    let (mut transform, mut orbit) = camera.into_inner();
    if viewport.focused && viewport.right_down {
        orbit.yaw -= viewport.pointer_delta.x * 0.006;
        orbit.pitch = (orbit.pitch - viewport.pointer_delta.y * 0.006).clamp(-1.48, -0.08);
    }
    if viewport.focused && viewport.scroll.y != 0.0 {
        orbit.distance = (orbit.distance * (-viewport.scroll.y * 0.0025).exp()).clamp(3.0, 100.0);
    }
    let rotation = Quat::from_euler(EulerRot::YXZ, orbit.yaw, orbit.pitch, 0.0);
    transform.translation = orbit.target + rotation * Vec3::new(0.0, 0.0, orbit.distance);
    transform.look_at(orbit.target, Vec3::Y);
}

#[allow(clippy::too_many_arguments)]
fn interact_with_map(
    camera: Single<(&Camera, &GlobalTransform), With<AtlasCamera>>,
    viewport: Res<ViewportState>,
    tool: Res<EditorTool>,
    brush: Res<TerrainBrushSettings>,
    selection: Res<LayerSelection>,
    settlement: Res<SettlementDraft>,
    features: Res<FeatureStore>,
    mut selected_feature: ResMut<SelectedFeature>,
    active_tile: Res<ActiveEditTile>,
    terrain: Res<TerrainStore>,
    mut stroke: ResMut<StrokeState>,
    mut status: ResMut<StatusLine>,
    mut bridge: ResMut<BridgeState>,
    mut commands: Commands,
) {
    let Some(cursor) = viewport.pointer else {
        stroke.last_stamp_uv = None;
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    let Some(tile) = terrain.tiles.get(&active_tile.0) else {
        return;
    };
    let Some((_point, uv)) = pick_terrain_surface(&ray, tile) else {
        stroke.last_stamp_uv = None;
        return;
    };

    if viewport.left_just_pressed {
        let position = surface_position_on_tile(active_tile.0, uv, tile.sample_bilinear(uv));
        if let Some(feature_id) =
            features.nearest_point(position.face, [position.u, position.v], 0.06)
        {
            selected_feature.0 = Some(feature_id);
            status.0 = features.features.get(&feature_id).map_or_else(
                || "Feature selected".into(),
                |feature| format!("Selected {}", feature.name),
            );
            bridge
                .events
                .push(MapEvent::SelectionChanged(Some(feature_id)));
            stroke.last_stamp_uv = Some(uv);
            return;
        }
    }

    if !viewport.left_down {
        stroke.last_stamp_uv = None;
        return;
    }
    let minimum_spacing = (brush.radius_cells / tile.edge_cells() as f32 * 0.18).max(0.002);
    if stroke.last_stamp_uv.is_some_and(|last| {
        let delta = Vec2::from_array(last) - Vec2::from_array(uv);
        delta.length() < minimum_spacing
    }) {
        return;
    }

    match *tool {
        EditorTool::Navigate => return,
        EditorTool::Sculpt => commands.trigger(SculptTerrain(HeightBrushStamp {
            tile: active_tile.0,
            center_uv: uv,
            radius_cells: brush.radius_cells,
            strength: brush.strength,
            falloff: brush.falloff,
            mode: brush.mode,
        })),
        EditorTool::Regions => {
            let (Some(layer), Some(region)) = (selection.layer, selection.region) else {
                status.0 = "Select a layer and region first".into();
                return;
            };
            commands.trigger(PaintRegion {
                stamp: RegionBrushStamp {
                    layer,
                    tile: active_tile.0,
                    region,
                    center_uv: uv,
                    radius_cells: brush.radius_cells,
                    falloff: brush.falloff,
                },
                edge_cells: tile.edge_cells() as u16,
            });
        }
        EditorTool::City if viewport.left_just_pressed => {
            commands.trigger(PlaceSettlement {
                name: settlement.name.clone(),
                position: surface_position_on_tile(active_tile.0, uv, tile.sample_bilinear(uv)),
                kind: settlement.kind,
                population: None,
                layer: selection.layer,
            });
        }
        EditorTool::City => return,
    }
    stroke.last_stamp_uv = Some(uv);
}

fn surface_position_on_tile(tile: TileId, uv: [f32; 2], altitude_m: f32) -> SurfacePosition {
    let (minimum, maximum) = tile.face_uv_bounds();
    SurfacePosition {
        face: tile.face,
        u: minimum[0] + uv[0] as f64 * (maximum[0] - minimum[0]),
        v: minimum[1] + uv[1] as f64 * (maximum[1] - minimum[1]),
        altitude_m,
    }
}

#[allow(clippy::too_many_arguments)]
fn refresh_snapshot(
    mut bridge: ResMut<BridgeState>,
    tool: Res<EditorTool>,
    brush: Res<TerrainBrushSettings>,
    selection: Res<LayerSelection>,
    selected: Res<SelectedFeature>,
    features: Res<FeatureStore>,
    layers: Res<LayerRegistry>,
    history: Res<EditHistory>,
    session: Res<ProjectSession>,
    status: Res<StatusLine>,
    camera: Single<&OrbitCamera, With<AtlasCamera>>,
) {
    let active_tool = match *tool {
        EditorTool::Navigate => MapTool::Navigate,
        EditorTool::Sculpt => MapTool::Sculpt,
        EditorTool::Regions => MapTool::Regions,
        EditorTool::City => MapTool::Settlement,
    };
    let available_layers = layers
        .ordering
        .iter()
        .filter_map(|id| layers.definitions.get(id))
        .map(|layer| LayerSummary {
            id: layer.id,
            name: layer.name.clone(),
            visible: layer.visible,
        })
        .collect::<Vec<_>>();
    let selected_layer = selection
        .layer
        .and_then(|id| layers.definitions.get(&id))
        .map(|layer| LayerSummary {
            id: layer.id,
            name: layer.name.clone(),
            visible: layer.visible,
        });
    let selected_feature = selected
        .0
        .and_then(|id| features.features.get(&id))
        .map(|feature| FeatureSummary {
            id: feature.id,
            name: feature.name.clone(),
            kind: feature_kind_name(&feature.kind).into(),
        });
    let orbit = camera.into_inner();
    bridge.snapshot = MapSnapshot {
        revision: bridge.revision,
        selected_feature,
        selected_layer,
        available_layers,
        active_tool,
        brush: BrushSettings {
            radius_cells: brush.radius_cells,
            strength: brush.strength,
        },
        camera: CameraSummary {
            distance: orbit.distance,
            yaw: orbit.yaw,
            pitch: orbit.pitch,
        },
        project_dirty: session.dirty,
        undo_available: history.undo_len() > 0,
        redo_available: history.redo_len() > 0,
        status: status.0.clone(),
    };
}

fn feature_kind_name(kind: &FeatureKind) -> &'static str {
    match kind {
        FeatureKind::Settlement { .. } => "Settlement",
        FeatureKind::River { .. } => "River",
        FeatureKind::Road => "Road",
        FeatureKind::SeaRoute => "Sea route",
        FeatureKind::Border => "Border",
        FeatureKind::Landmark => "Landmark",
    }
}

fn finish_input_frame(mut viewport: ResMut<ViewportState>) {
    viewport.pointer_delta = Vec2::ZERO;
    viewport.scroll = Vec2::ZERO;
    viewport.left_just_pressed = false;
}

fn on_feature_created(
    event: On<FeatureCreated>,
    mut selected: ResMut<SelectedFeature>,
    mut bridge: ResMut<BridgeState>,
) {
    selected.0 = Some(event.feature);
    bridge
        .events
        .push(MapEvent::SelectionChanged(Some(event.feature)));
}

fn on_feature_failed(event: On<FeatureEditFailed>, mut bridge: ResMut<BridgeState>) {
    bridge.events.push(MapEvent::CommandRejected {
        error: event.0.clone(),
    });
}

fn on_terrain_changed(_event: On<TerrainChanged>, mut bridge: ResMut<BridgeState>) {
    bridge.revision = bridge.revision.wrapping_add(1);
    let revision = bridge.revision;
    bridge.events.push(MapEvent::CommandAccepted { revision });
}

fn on_history_outcome(event: On<HistoryOutcome>, mut bridge: ResMut<BridgeState>) {
    match &*event {
        HistoryOutcome::Undid | HistoryOutcome::Redid => {
            bridge.revision = bridge.revision.wrapping_add(1);
            let revision = bridge.revision;
            bridge.events.push(MapEvent::CommandAccepted { revision });
        }
        HistoryOutcome::NothingToUndo => bridge.events.push(MapEvent::CommandRejected {
            error: "Nothing to undo".into(),
        }),
        HistoryOutcome::NothingToRedo => bridge.events.push(MapEvent::CommandRejected {
            error: "Nothing to redo".into(),
        }),
        HistoryOutcome::Failed(error) => bridge.events.push(MapEvent::CommandRejected {
            error: error.clone(),
        }),
    }
}

fn on_project_saved(event: On<ProjectSaved>, mut bridge: ResMut<BridgeState>) {
    bridge.events.push(MapEvent::ProjectSaved {
        path: event.path.display().to_string(),
    });
}

fn on_project_loaded(event: On<ProjectLoaded>, mut bridge: ResMut<BridgeState>) {
    bridge.events.push(MapEvent::ProjectLoaded {
        path: event.path.display().to_string(),
    });
}

fn on_project_failed(event: On<ProjectIoFailed>, mut bridge: ResMut<BridgeState>) {
    bridge.events.push(MapEvent::ProjectIoFailed {
        operation: event.operation.into(),
        error: event.error.clone(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite_or_non_positive_brushes() {
        assert!(valid_brush(BrushSettings::default()));
        assert!(!valid_brush(BrushSettings {
            radius_cells: 0.0,
            strength: 1.0,
        }));
        assert!(!valid_brush(BrushSettings {
            radius_cells: 1.0,
            strength: f32::NAN,
        }));
    }

    #[test]
    fn tile_coordinates_stay_framework_neutral_at_the_boundary() {
        let position = surface_position_on_tile(TileId::ROOT, [0.5, 0.5], 42.0);
        assert_eq!([position.u, position.v], [0.0, 0.0]);
        assert_eq!(position.altitude_m, 42.0);
    }
}
