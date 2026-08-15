use bevy::prelude::*;
use bevy_egui::EguiPlugin;
use kovia_atlas::{
    camera::MapCameraPlugin, change_tracking::ChangeTrackingPlugin, coordinates::PlanetPlugin,
    editor::EditorStatePlugin, erosion::ErosionPlugin, feature_render::FeatureRenderPlugin,
    features::FeaturePlugin, history::HistoryPlugin, hydrology::HydrologyPlugin,
    interaction::MapInteractionPlugin, layers::LayerPlugin,
    overlay_render::HydrologyOverlayRenderPlugin, persistence::ProjectPersistencePlugin,
    starter_world::StarterWorldPlugin, streaming::TileResidencyPlugin, terrain::TerrainPlugin,
    terrain_render::TerrainRenderPlugin, ui::AtlasUiPlugin,
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::srgb(0.018, 0.022, 0.028)))
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Kovia Atlas".into(),
                resolution: (1440, 900).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        // Product composition is explicit. Every line is a removable
        // capability; no Kovia god-plugin installs the application for us.
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
        .add_plugins(MapCameraPlugin)
        .add_plugins(TerrainRenderPlugin)
        .add_plugins(FeatureRenderPlugin)
        .add_plugins(HydrologyOverlayRenderPlugin)
        .add_plugins(MapInteractionPlugin)
        .add_plugins(AtlasUiPlugin)
        .add_plugins(StarterWorldPlugin)
        .run();
}
