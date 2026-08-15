use bevy::prelude::*;

use crate::{
    editor::ActiveEditTile,
    hydrology::HydrologyCache,
    terrain::TerrainStore,
    terrain_render::{DEMO_TILE_SIZE, HEIGHT_DISPLAY_SCALE},
};

pub struct HydrologyOverlayRenderPlugin;

impl Plugin for HydrologyOverlayRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_rivers);
    }
}

fn draw_rivers(
    terrain: Res<TerrainStore>,
    hydrology: Res<HydrologyCache>,
    active: Res<ActiveEditTile>,
    mut gizmos: Gizmos,
) {
    let Some(tile) = terrain.tiles.get(&active.0) else {
        return;
    };
    let Some(overlay) = hydrology.tiles.get(&active.0) else {
        return;
    };
    let edge = overlay.edge_samples as usize;
    if edge == 0 || edge != tile.edge_samples as usize || overlay.source_revision != tile.revision {
        return;
    }
    for segment in &overlay.segments {
        let position = |index: usize| {
            let x = index % edge;
            let y = index / edge;
            let u = x as f32 / (edge - 1) as f32;
            let v = y as f32 / (edge - 1) as f32;
            Vec3::new(
                (u - 0.5) * DEMO_TILE_SIZE,
                tile.get(x, y) * HEIGHT_DISPLAY_SCALE + 0.035,
                (v - 0.5) * DEMO_TILE_SIZE,
            )
        };
        let alpha = (segment.discharge / 500.0).clamp(0.35, 1.0);
        gizmos.line(
            position(segment.from as usize),
            position(segment.to as usize),
            Color::srgba(0.18, 0.64, 0.94, alpha),
        );
    }
}
