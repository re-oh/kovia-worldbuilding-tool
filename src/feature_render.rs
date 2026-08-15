use bevy::prelude::*;

use crate::{
    coordinates::SurfacePosition,
    editor::{ActiveEditTile, SelectedFeature},
    features::{FeatureGeometry, FeatureKind, FeatureStore, SettlementKind},
    terrain::TerrainStore,
    terrain_render::{DEMO_TILE_SIZE, HEIGHT_DISPLAY_SCALE},
};

pub struct FeatureRenderPlugin;

impl Plugin for FeatureRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, draw_features);
    }
}

fn draw_features(
    features: Res<FeatureStore>,
    terrain: Res<TerrainStore>,
    active: Res<ActiveEditTile>,
    selected: Res<SelectedFeature>,
    mut gizmos: Gizmos,
) {
    let Some(tile) = terrain.tiles.get(&active.0) else {
        return;
    };
    for feature in features.features.values() {
        let FeatureGeometry::Point(position) = feature.geometry else {
            continue;
        };
        let Some(uv) = tile_uv(active.0, position) else {
            continue;
        };
        let point = Vec3::new(
            (uv[0] - 0.5) * DEMO_TILE_SIZE,
            tile.sample_bilinear(uv) * HEIGHT_DISPLAY_SCALE + 0.11,
            (uv[1] - 0.5) * DEMO_TILE_SIZE,
        );
        let is_selected = selected.0 == Some(feature.id);
        let color = if is_selected {
            Color::srgb(1.0, 0.84, 0.22)
        } else {
            feature_color(&feature.kind)
        };
        let radius = if is_selected { 0.17 } else { 0.11 };
        gizmos.sphere(Isometry3d::from_translation(point), radius, color);
        gizmos.line(point, point + Vec3::Y * 0.45, color);
        let arm = if is_selected { 0.24 } else { 0.16 };
        gizmos.line(point - Vec3::X * arm, point + Vec3::X * arm, color);
        gizmos.line(point - Vec3::Z * arm, point + Vec3::Z * arm, color);
    }
}

fn tile_uv(tile: crate::coordinates::TileId, position: SurfacePosition) -> Option<[f32; 2]> {
    if tile.face != position.face {
        return None;
    }
    let (minimum, maximum) = tile.face_uv_bounds();
    if position.u < minimum[0]
        || position.u > maximum[0]
        || position.v < minimum[1]
        || position.v > maximum[1]
    {
        return None;
    }
    Some([
        ((position.u - minimum[0]) / (maximum[0] - minimum[0])) as f32,
        ((position.v - minimum[1]) / (maximum[1] - minimum[1])) as f32,
    ])
}

fn feature_color(kind: &FeatureKind) -> Color {
    match kind {
        FeatureKind::Settlement { kind, .. } => match kind {
            SettlementKind::Capital => Color::srgb(0.95, 0.35, 0.30),
            SettlementKind::City | SettlementKind::Port => Color::srgb(0.94, 0.72, 0.28),
            SettlementKind::Town => Color::srgb(0.52, 0.86, 0.72),
            SettlementKind::Village | SettlementKind::Hamlet => Color::srgb(0.72, 0.82, 0.76),
        },
        FeatureKind::River { .. } => Color::srgb(0.20, 0.62, 0.95),
        FeatureKind::Road | FeatureKind::SeaRoute => Color::srgb(0.83, 0.68, 0.46),
        FeatureKind::Border => Color::srgb(0.86, 0.35, 0.50),
        FeatureKind::Landmark => Color::srgb(0.70, 0.50, 0.95),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::{CubeFace, TileId};

    #[test]
    fn surface_position_maps_into_resident_tile() {
        let child = TileId::new(CubeFace::PositiveY, 1, 1, 0).unwrap();
        let position = SurfacePosition {
            face: CubeFace::PositiveY,
            u: 0.5,
            v: -0.5,
            altitude_m: 0.0,
        };
        assert_eq!(tile_uv(child, position), Some([0.5, 0.5]));
    }
}
