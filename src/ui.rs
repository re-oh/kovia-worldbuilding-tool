use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};
use egui_phosphor::regular;

use crate::{
    brush::BrushFalloff,
    camera::ViewportInputCapture,
    change_tracking::DirtyTracker,
    editor::{
        ActiveEditTile, EditorDraftNames, EditorTool, LayerSelection, SelectedFeature,
        SettlementDraft, StatusLine, TerrainBrushSettings,
    },
    erosion::{ErosionFailed, HydraulicErosionSettings, RunErosion},
    features::{
        DeleteFeature, FeatureCreated, FeatureEditFailed, FeatureKind, FeatureStore,
        SettlementKind, UpdateSettlement,
    },
    history::{EditHistory, HistoryOutcome, Redo, Undo},
    hydrology::{ComputeHydrology, HydrologyCache, HydrologyFailed, HydrologyUpdated},
    layers::{
        ConfigureLayer, CreateLayer, CreateRegion, LayerCreated, LayerEditFailed, LayerRegistry,
        LayerStore, RegionCreated,
    },
    persistence::{
        LoadProject, ProjectIoFailed, ProjectLoaded, ProjectPath, ProjectSaved, ProjectSession,
        SaveProject,
    },
    terrain::{SculptMode, TerrainEditFailed, TerrainStore},
};

#[derive(Debug, Default, Resource)]
struct UiStyleState {
    configured: bool,
    confirm_discard: bool,
}

/// egui adapter only. Widgets emit domain events and never mutate canonical
/// terrain, layer, feature, history, or compute stores.
pub struct AtlasUiPlugin;

impl Plugin for AtlasUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiStyleState>()
            .add_systems(EguiPrimaryContextPass, editor_ui)
            .add_observer(select_created_layer)
            .add_observer(select_created_region)
            .add_observer(show_history_outcome)
            .add_observer(show_hydrology_outcome)
            .add_observer(show_hydrology_failure)
            .add_observer(show_erosion_failure)
            .add_observer(show_terrain_failure)
            .add_observer(show_layer_failure)
            .add_observer(select_created_feature)
            .add_observer(show_feature_failure)
            .add_observer(show_project_saved)
            .add_observer(show_project_loaded)
            .add_observer(show_project_failure);
    }
}

#[allow(clippy::too_many_arguments)]
#[derive(SystemParam)]
struct EditorUiParams<'w, 's> {
    style: ResMut<'w, UiStyleState>,
    tool: ResMut<'w, EditorTool>,
    brush: ResMut<'w, TerrainBrushSettings>,
    selection: ResMut<'w, LayerSelection>,
    settlement: ResMut<'w, SettlementDraft>,
    selected_feature: ResMut<'w, SelectedFeature>,
    draft_names: ResMut<'w, EditorDraftNames>,
    status: ResMut<'w, StatusLine>,
    active: Res<'w, ActiveEditTile>,
    terrain: Res<'w, TerrainStore>,
    registry: Res<'w, LayerRegistry>,
    layer_store: Res<'w, LayerStore>,
    features: Res<'w, FeatureStore>,
    dirty: Res<'w, DirtyTracker>,
    history: Res<'w, EditHistory>,
    hydrology: Res<'w, HydrologyCache>,
    project_path: ResMut<'w, ProjectPath>,
    project_session: Res<'w, ProjectSession>,
    capture: ResMut<'w, ViewportInputCapture>,
    commands: Commands<'w, 's>,
}

fn editor_ui(mut contexts: EguiContexts, params: EditorUiParams) -> Result {
    let EditorUiParams {
        mut style,
        mut tool,
        mut brush,
        mut selection,
        mut settlement,
        mut selected_feature,
        mut draft_names,
        mut status,
        active,
        terrain,
        registry,
        layer_store,
        features,
        dirty,
        history,
        hydrology,
        mut project_path,
        project_session,
        mut capture,
        mut commands,
    } = params;
    let ctx = contexts.ctx_mut()?;
    if !style.configured {
        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        ctx.set_fonts(fonts);
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(25, 28, 34);
        visuals.window_fill = egui::Color32::from_rgb(25, 28, 34);
        ctx.set_visuals(visuals);
        style.configured = true;
    }

    let mut viewport_ui = egui::Ui::new(
        ctx.clone(),
        "atlas_viewport".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    );

    egui::Panel::top("top_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading(format!("{}  Kovia Atlas", regular::MAP_TRIFOLD));
            ui.separator();
            if ui
                .button(format!("{} Undo", regular::ARROW_COUNTER_CLOCKWISE))
                .clicked()
            {
                commands.trigger(Undo);
            }
            if ui
                .button(format!("{} Redo", regular::ARROW_CLOCKWISE))
                .clicked()
            {
                commands.trigger(Redo);
            }
            ui.separator();
            if ui
                .button(format!("{} Compute rivers", regular::WAVES))
                .clicked()
            {
                commands.trigger(ComputeHydrology {
                    tile: active.0,
                    rainfall: 1.0,
                    river_threshold: 90.0,
                });
                status.0 = "Computing hydrology".into();
            }
            if ui.button(format!("{} Erode", regular::DROP)).clicked() {
                commands.trigger(RunErosion {
                    tile: active.0,
                    settings: HydraulicErosionSettings {
                        iterations: 24,
                        ..Default::default()
                    },
                });
                status.0 = "Running 24 CPU erosion iterations".into();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(&status.0);
            });
        });
    });

    egui::Panel::top("project_bar").show(&mut viewport_ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(if project_session.dirty {
                "Project *"
            } else {
                "Project"
            });
            ui.add(
                egui::TextEdit::singleline(&mut project_path.0)
                    .desired_width(320.0)
                    .hint_text("path/to/world.kovia.json"),
            );
            if ui
                .button(format!("{} Save", regular::FLOPPY_DISK))
                .clicked()
            {
                commands.trigger(SaveProject);
                style.confirm_discard = false;
            }
            if ui
                .button(format!("{} Load", regular::FOLDER_OPEN))
                .clicked()
            {
                if project_session.dirty {
                    style.confirm_discard = true;
                    status.0 = "Unsaved changes: confirm load to discard them".into();
                } else {
                    commands.trigger(LoadProject);
                }
            }
            if style.confirm_discard
                && ui
                    .button(format!("{} Discard & load", regular::WARNING))
                    .clicked()
            {
                commands.trigger(LoadProject);
                style.confirm_discard = false;
            }
        });
    });

    egui::Panel::left("tools")
        .default_size(280.0)
        .resizable(true)
        .show(&mut viewport_ui, |ui| {
            ui.heading("Tools");
            ui.horizontal_wrapped(|ui| {
                tool_button(
                    ui,
                    &mut tool,
                    EditorTool::Navigate,
                    regular::HAND,
                    "Navigate",
                );
                tool_button(
                    ui,
                    &mut tool,
                    EditorTool::Sculpt,
                    regular::MOUNTAINS,
                    "Terrain",
                );
                tool_button(
                    ui,
                    &mut tool,
                    EditorTool::Regions,
                    regular::PAINT_BRUSH,
                    "Regions",
                );
                tool_button(
                    ui,
                    &mut tool,
                    EditorTool::City,
                    regular::BUILDINGS,
                    "Cities",
                );
            });
            ui.separator();
            match *tool {
                EditorTool::Navigate => {
                    ui.label("Right-drag to orbit. Scroll to zoom.");
                }
                EditorTool::Sculpt => sculpt_panel(ui, &mut brush),
                EditorTool::Regions => region_panel(
                    ui,
                    &mut selection,
                    &mut draft_names,
                    &registry,
                    &mut commands,
                ),
                EditorTool::City => city_panel(
                    ui,
                    &mut settlement,
                    &features,
                    &mut selected_feature,
                    &mut commands,
                ),
            }
            ui.separator();
            ui.collapsing("World diagnostics", |ui| {
                ui.label(format!("Terrain tiles: {}", terrain.tiles.len()));
                ui.label(format!("Semantic layers: {}", registry.definitions.len()));
                ui.label(format!(
                    "Layer masks: {}",
                    layer_store.categorical_tiles.len()
                ));
                ui.label(format!("Features: {}", features.features.len()));
                ui.label(format!("Dirty tiles: {}", dirty.tiles.len()));
                ui.label(format!(
                    "Undo / redo: {} / {}",
                    history.undo_len(),
                    history.redo_len()
                ));
                let rivers = hydrology
                    .tiles
                    .get(&active.0)
                    .map_or(0, |tile| tile.segments.len());
                ui.label(format!("River segments: {rivers}"));
            });
        });

    capture.pointer = ctx.egui_wants_pointer_input() || ctx.is_pointer_over_egui();
    Ok(())
}

fn tool_button(
    ui: &mut egui::Ui,
    selected: &mut EditorTool,
    value: EditorTool,
    icon: &str,
    label: &str,
) {
    if ui
        .selectable_label(*selected == value, format!("{icon} {label}"))
        .clicked()
    {
        *selected = value;
    }
}

fn sculpt_panel(ui: &mut egui::Ui, brush: &mut TerrainBrushSettings) {
    ui.heading("Terrain brush");
    ui.horizontal_wrapped(|ui| {
        ui.selectable_value(&mut brush.mode, SculptMode::Raise, "Raise");
        ui.selectable_value(&mut brush.mode, SculptMode::Lower, "Lower");
        ui.selectable_value(&mut brush.mode, SculptMode::Smooth, "Smooth");
        let flatten = matches!(brush.mode, SculptMode::Flatten { .. });
        if ui.selectable_label(flatten, "Flatten").clicked() {
            brush.mode = SculptMode::Flatten { target_m: 250.0 };
        }
    });
    ui.add(egui::Slider::new(&mut brush.radius_cells, 1.0..=40.0).text("Radius (cells)"));
    ui.add(egui::Slider::new(&mut brush.strength, 1.0..=160.0).text("Strength"));
    ui.horizontal(|ui| {
        ui.label("Falloff");
        ui.selectable_value(&mut brush.falloff, BrushFalloff::Constant, "Hard");
        ui.selectable_value(&mut brush.falloff, BrushFalloff::Linear, "Linear");
        ui.selectable_value(&mut brush.falloff, BrushFalloff::Smooth, "Smooth");
    });
}

fn region_panel(
    ui: &mut egui::Ui,
    selection: &mut LayerSelection,
    drafts: &mut EditorDraftNames,
    registry: &LayerRegistry,
    commands: &mut Commands,
) {
    ui.heading("Semantic layers");
    for id in &registry.ordering {
        let definition = &registry.definitions[id];
        if ui
            .selectable_label(selection.layer == Some(*id), &definition.name)
            .clicked()
        {
            selection.layer = Some(*id);
            selection.region = definition.regions.keys().next().copied();
        }
    }
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut drafts.layer);
        if ui.button(format!("{} Layer", regular::PLUS)).clicked() {
            commands.trigger(CreateLayer {
                name: drafts.layer.clone(),
            });
        }
    });
    ui.separator();
    let Some(layer_id) = selection.layer else {
        ui.label("Create or select a layer.");
        return;
    };
    let Some(layer) = registry.definitions.get(&layer_id) else {
        return;
    };
    let mut visible = layer.visible;
    let mut locked = layer.locked;
    let mut opacity = layer.opacity;
    ui.horizontal(|ui| {
        ui.checkbox(&mut visible, "Visible");
        ui.checkbox(&mut locked, "Locked");
    });
    ui.add(egui::Slider::new(&mut opacity, 0.0..=1.0).text("Overlay opacity"));
    if visible != layer.visible || locked != layer.locked || opacity != layer.opacity {
        commands.trigger(ConfigureLayer {
            layer: layer_id,
            visible: (visible != layer.visible).then_some(visible),
            locked: (locked != layer.locked).then_some(locked),
            opacity: (opacity != layer.opacity).then_some(opacity),
        });
    }
    ui.label("Regions");
    for (code, region) in &layer.regions {
        let color = egui::Color32::from_rgba_unmultiplied(
            region.color_rgba[0],
            region.color_rgba[1],
            region.color_rgba[2],
            region.color_rgba[3],
        );
        ui.horizontal(|ui| {
            ui.colored_label(color, "●");
            if ui
                .selectable_label(selection.region == Some(*code), &region.name)
                .clicked()
            {
                selection.region = Some(*code);
            }
        });
    }
    ui.horizontal(|ui| {
        ui.text_edit_singleline(&mut drafts.region);
        if ui.button(format!("{} Region", regular::PLUS)).clicked() {
            const PALETTE: [[u8; 4]; 6] = [
                [224, 105, 72, 255],
                [78, 150, 196, 255],
                [125, 174, 92, 255],
                [181, 125, 201, 255],
                [220, 176, 72, 255],
                [81, 184, 166, 255],
            ];
            commands.trigger(CreateRegion {
                layer: layer_id,
                name: drafts.region.clone(),
                color_rgba: PALETTE[layer.regions.len() % PALETTE.len()],
            });
        }
    });
}

fn city_panel(
    ui: &mut egui::Ui,
    settlement: &mut SettlementDraft,
    features: &FeatureStore,
    selected: &mut SelectedFeature,
    commands: &mut Commands,
) {
    ui.heading(if selected.0.is_some() {
        "Edit settlement"
    } else {
        "Place settlement"
    });
    ui.text_edit_singleline(&mut settlement.name);
    egui::ComboBox::from_label("Type")
        .selected_text(format!("{:?}", settlement.kind))
        .show_ui(ui, |ui| {
            for kind in [
                SettlementKind::Hamlet,
                SettlementKind::Village,
                SettlementKind::Town,
                SettlementKind::City,
                SettlementKind::Capital,
                SettlementKind::Port,
            ] {
                ui.selectable_value(&mut settlement.kind, kind, format!("{kind:?}"));
            }
        });
    if let Some(feature) = selected.0 {
        ui.horizontal(|ui| {
            if ui.button(format!("{} Update", regular::CHECK)).clicked() {
                commands.trigger(UpdateSettlement {
                    feature,
                    name: settlement.name.clone(),
                    kind: settlement.kind,
                    population: None,
                });
            }
            if ui.button(format!("{} Delete", regular::TRASH)).clicked() {
                commands.trigger(DeleteFeature { feature });
                selected.0 = None;
            }
            if ui.button("Place another").clicked() {
                selected.0 = None;
            }
        });
    } else {
        ui.label("Click the terrain to place it. Click a marker to select it.");
    }

    ui.separator();
    ui.heading("Settlements");
    let rows: Vec<_> = features
        .features
        .values()
        .filter_map(|feature| {
            let FeatureKind::Settlement { kind, .. } = feature.kind else {
                return None;
            };
            Some((feature.id, feature.name.clone(), kind))
        })
        .collect();
    if rows.is_empty() {
        ui.label("No settlements placed yet.");
    }
    for (id, name, kind) in rows {
        if ui
            .selectable_label(selected.0 == Some(id), format!("{name}  ·  {kind:?}"))
            .clicked()
        {
            selected.0 = Some(id);
            settlement.name = name;
            settlement.kind = kind;
        }
    }
}

fn select_created_feature(
    event: On<FeatureCreated>,
    mut selected: ResMut<SelectedFeature>,
    mut status: ResMut<StatusLine>,
) {
    selected.0 = Some(event.feature);
    status.0 = "Placed settlement".into();
}

fn show_feature_failure(event: On<FeatureEditFailed>, mut status: ResMut<StatusLine>) {
    status.0 = event.0.clone();
}

fn show_project_saved(event: On<ProjectSaved>, mut status: ResMut<StatusLine>) {
    status.0 = format!("Saved {}", event.path.display());
}

fn show_project_loaded(event: On<ProjectLoaded>, mut status: ResMut<StatusLine>) {
    status.0 = format!("Loaded {}", event.path.display());
}

fn show_project_failure(event: On<ProjectIoFailed>, mut status: ResMut<StatusLine>) {
    status.0 = format!("Could not {} project: {}", event.operation, event.error);
}

fn select_created_layer(
    event: On<LayerCreated>,
    mut selection: ResMut<LayerSelection>,
    mut status: ResMut<StatusLine>,
) {
    selection.layer = Some(event.layer);
    selection.region = None;
    status.0 = "Created layer".into();
}

fn select_created_region(
    event: On<RegionCreated>,
    mut selection: ResMut<LayerSelection>,
    mut status: ResMut<StatusLine>,
) {
    selection.layer = Some(event.layer);
    selection.region = Some(event.region);
    status.0 = "Created region".into();
}

fn show_history_outcome(event: On<HistoryOutcome>, mut status: ResMut<StatusLine>) {
    status.0 = match &*event {
        HistoryOutcome::Undid => "Undid edit".into(),
        HistoryOutcome::Redid => "Redid edit".into(),
        HistoryOutcome::NothingToUndo => "Nothing to undo".into(),
        HistoryOutcome::NothingToRedo => "Nothing to redo".into(),
        HistoryOutcome::Failed(error) => error.to_string(),
    };
}

fn show_hydrology_outcome(event: On<HydrologyUpdated>, mut status: ResMut<StatusLine>) {
    status.0 = format!("Computed {} river segments", event.river_segments);
}

fn show_hydrology_failure(_event: On<HydrologyFailed>, mut status: ResMut<StatusLine>) {
    status.0 = "Hydrology tile is not loaded".into();
}

fn show_erosion_failure(_event: On<ErosionFailed>, mut status: ResMut<StatusLine>) {
    status.0 = "Erosion tile is not loaded".into();
}

fn show_terrain_failure(event: On<TerrainEditFailed>, mut status: ResMut<StatusLine>) {
    status.0 = event.0.to_string();
}

fn show_layer_failure(event: On<LayerEditFailed>, mut status: ResMut<StatusLine>) {
    status.0 = event.0.to_string();
}
