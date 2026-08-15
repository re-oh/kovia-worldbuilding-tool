use bevy::prelude::*;
use kovia_atlas::{
    brush::BrushFalloff,
    coordinates::TileId,
    history::{HistoryPlugin, Undo},
    layers::{
        LayerDefinition, LayerPlugin, LayerRegistry, LayerStore, PaintRegion, RegionBrushStamp,
    },
    terrain::{HeightBrushStamp, SculptMode, SculptTerrain, TerrainPlugin, TerrainStore},
};

#[test]
fn terrain_plugin_runs_without_the_editor_or_atlas_root() {
    let mut app = App::new();
    app.add_plugins(TerrainPlugin);
    app.world_mut()
        .resource_mut::<TerrainStore>()
        .ensure_flat(TileId::ROOT, 9, 0.0);

    app.world_mut().trigger(SculptTerrain(HeightBrushStamp {
        tile: TileId::ROOT,
        center_uv: [0.5, 0.5],
        radius_cells: 2.0,
        strength: 100.0,
        falloff: BrushFalloff::Smooth,
        mode: SculptMode::Raise,
    }));

    assert!(app.world().resource::<TerrainStore>().tiles[&TileId::ROOT].max_height_m > 0.0);
}

#[test]
fn history_is_optional_and_uses_domain_owned_undo_behavior() {
    let mut app = App::new();
    app.add_plugins((TerrainPlugin, HistoryPlugin));
    app.world_mut()
        .resource_mut::<TerrainStore>()
        .ensure_flat(TileId::ROOT, 9, 0.0);

    app.world_mut().trigger(SculptTerrain(HeightBrushStamp {
        tile: TileId::ROOT,
        center_uv: [0.5, 0.5],
        radius_cells: 2.0,
        strength: 100.0,
        falloff: BrushFalloff::Smooth,
        mode: SculptMode::Raise,
    }));
    assert!(app.world().resource::<TerrainStore>().tiles[&TileId::ROOT].max_height_m > 0.0);

    app.world_mut().trigger(Undo);
    app.update();
    assert_eq!(
        app.world().resource::<TerrainStore>().tiles[&TileId::ROOT].max_height_m,
        0.0
    );
}

#[test]
fn layer_plugin_runs_without_terrain_features_or_ui() {
    let mut app = App::new();
    app.add_plugins(LayerPlugin);

    let (layer, region) = {
        let mut registry = app.world_mut().resource_mut::<LayerRegistry>();
        let mut definition = LayerDefinition::categorical("Kingdoms");
        let region = definition.add_region("Kovia", [224, 105, 72, 255]);
        (registry.insert(definition), region)
    };

    app.world_mut().trigger(PaintRegion {
        stamp: RegionBrushStamp {
            layer,
            tile: TileId::ROOT,
            region,
            center_uv: [0.5, 0.5],
            radius_cells: 3.0,
            falloff: BrushFalloff::Constant,
        },
        edge_cells: 8,
    });

    assert_eq!(
        app.world().resource::<LayerStore>().categorical_tiles.len(),
        1
    );
}
