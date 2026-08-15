use bevy::prelude::*;

use crate::{
    coordinates::TileId,
    editor::LayerSelection,
    layers::{LayerDefinition, LayerRegistry},
    terrain::{DEFAULT_TILE_CELLS, TerrainStore},
};

/// Optional demonstration content. Removing it yields an empty editor without
/// changing any domain or runtime plugin.
pub struct StarterWorldPlugin;

impl Plugin for StarterWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_starter_world);
    }
}

fn create_starter_world(
    mut terrain: ResMut<TerrainStore>,
    mut layers: ResMut<LayerRegistry>,
    mut selection: ResMut<LayerSelection>,
) {
    let tile = terrain.ensure_flat(TileId::ROOT, DEFAULT_TILE_CELLS + 1, -180.0);
    let edge = tile.edge_samples as usize;
    for y in 0..edge {
        for x in 0..edge {
            let u = x as f32 / (edge - 1) as f32;
            let v = y as f32 / (edge - 1) as f32;
            let dx = u - 0.48;
            let dy = v - 0.52;
            let continental = (1.0 - (dx * dx * 1.4 + dy * dy).sqrt() * 2.2).max(0.0);
            let ridge_axis = (u - (0.62 + (v * 5.0).sin() * 0.035)).abs();
            let ridge = (-ridge_axis * ridge_axis / 0.004).exp()
                * (v * std::f32::consts::PI).sin().max(0.0);
            let rolling = ((u * 19.0).sin() * (v * 17.0).cos()) * 45.0;
            tile.set(
                x,
                y,
                -220.0 + continental.powf(1.7) * 950.0 + ridge * 1_650.0 + rolling * continental,
            );
        }
    }
    tile.touch();

    let mut kingdoms = LayerDefinition::categorical("Kingdoms");
    let kovia = kingdoms.add_region("Kovia", [224, 105, 72, 255]);
    let layer = layers.insert(kingdoms);
    selection.layer = Some(layer);
    selection.region = Some(kovia);
}
