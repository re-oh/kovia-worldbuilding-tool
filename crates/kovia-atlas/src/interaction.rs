use bevy::prelude::*;

use crate::{
    camera::{AtlasCamera, ViewportInputCapture},
    coordinates::SurfacePosition,
    editor::{
        ActiveEditTile, EditorTool, LayerSelection, SelectedFeature, SettlementDraft, StatusLine,
        StrokeState, TerrainBrushSettings,
    },
    features::{FeatureStore, PlaceSettlement},
    layers::{PaintRegion, RegionBrushStamp},
    terrain::{HeightBrushStamp, HeightTile, SculptTerrain, TerrainStore},
    terrain_render::{DEMO_TILE_SIZE, HEIGHT_DISPLAY_SCALE},
};

/// Translates viewport gestures into domain events. It never mutates canonical
/// terrain, layers, or features directly.
pub struct MapInteractionPlugin;

impl Plugin for MapInteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, interact_with_map);
    }
}

#[allow(clippy::too_many_arguments)]
fn interact_with_map(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform), With<AtlasCamera>>,
    buttons: Res<ButtonInput<MouseButton>>,
    capture: Res<ViewportInputCapture>,
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
    mut gizmos: Gizmos,
    mut commands: Commands,
) {
    if capture.pointer {
        stroke.last_stamp_uv = None;
        return;
    }
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let (camera, camera_transform) = *camera;
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor) else {
        return;
    };
    let Some(tile) = terrain.tiles.get(&active_tile.0) else {
        return;
    };
    let Some((point, uv)) = pick_terrain_surface(&ray, tile) else {
        stroke.last_stamp_uv = None;
        return;
    };
    let radius_world = brush.radius_cells / tile.edge_cells() as f32 * DEMO_TILE_SIZE;
    gizmos.circle(
        Isometry3d::new(
            point + Vec3::Y * 0.035,
            Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
        ),
        radius_world,
        Color::srgba(0.95, 0.82, 0.34, 0.9),
    );

    if !buttons.pressed(MouseButton::Left) {
        stroke.last_stamp_uv = None;
        return;
    }
    let minimum_spacing = (brush.radius_cells / tile.edge_cells() as f32 * 0.18).max(0.002);
    if stroke.last_stamp_uv.is_some_and(|last| {
        let dx = last[0] - uv[0];
        let dy = last[1] - uv[1];
        (dx * dx + dy * dy).sqrt() < minimum_spacing
    }) {
        return;
    }

    match *tool {
        EditorTool::Sculpt => {
            commands.trigger(SculptTerrain(HeightBrushStamp {
                tile: active_tile.0,
                center_uv: uv,
                radius_cells: brush.radius_cells,
                strength: brush.strength,
                falloff: brush.falloff,
                mode: brush.mode,
            }));
        }
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
        EditorTool::City if buttons.just_pressed(MouseButton::Left) => {
            let position = surface_position_on_tile(active_tile.0, uv, tile.sample_bilinear(uv));
            if let Some(feature_id) =
                features.nearest_point(position.face, [position.u, position.v], 0.045)
            {
                selected_feature.0 = Some(feature_id);
                if let Some(feature) = features.features.get(&feature_id) {
                    status.0 = format!("Selected {}", feature.name);
                }
                stroke.last_stamp_uv = Some(uv);
                return;
            }
            selected_feature.0 = None;
            commands.trigger(PlaceSettlement {
                name: settlement.name.clone(),
                position,
                kind: settlement.kind,
                population: None,
                layer: selection.layer,
            });
        }
        _ => return,
    }
    stroke.last_stamp_uv = Some(uv);
}

fn surface_position_on_tile(
    tile: crate::coordinates::TileId,
    uv: [f32; 2],
    altitude_m: f32,
) -> SurfacePosition {
    let (minimum, maximum) = tile.face_uv_bounds();
    SurfacePosition {
        face: tile.face,
        u: minimum[0] + uv[0] as f64 * (maximum[0] - minimum[0]),
        v: minimum[1] + uv[1] as f64 * (maximum[1] - minimum[1]),
        altitude_m,
    }
}

/// Intersects a viewport ray with the rendered bilinear height field. A coarse
/// march finds the first surface crossing and a binary search refines it.
pub fn pick_terrain_surface(ray: &Ray3d, tile: &HeightTile) -> Option<(Vec3, [f32; 2])> {
    let direction = *ray.direction;
    let half = DEMO_TILE_SIZE * 0.5;
    let (x_start, x_end) = ray_axis_interval(ray.origin.x, direction.x, -half, half)?;
    let (z_start, z_end) = ray_axis_interval(ray.origin.z, direction.z, -half, half)?;
    let minimum_y = tile.min_height_m * HEIGHT_DISPLAY_SCALE;
    let maximum_y = tile.max_height_m * HEIGHT_DISPLAY_SCALE;
    let (y_start, y_end) = ray_axis_interval(ray.origin.y, direction.y, minimum_y, maximum_y)?;
    let start = x_start.max(z_start).max(y_start).max(0.0);
    let end = x_end.min(z_end).min(y_end);
    if !start.is_finite() || !end.is_finite() || end < start {
        return None;
    }

    let gap = |distance: f32| {
        let point = ray.get_point(distance);
        let uv = [
            (point.x + half) / DEMO_TILE_SIZE,
            (point.z + half) / DEMO_TILE_SIZE,
        ];
        point.y - tile.sample_bilinear(uv) * HEIGHT_DISPLAY_SCALE
    };

    const MARCH_STEPS: usize = 256;
    let mut previous_distance = start;
    let mut previous_gap = gap(start);
    for step in 1..=MARCH_STEPS {
        let distance = start + (end - start) * step as f32 / MARCH_STEPS as f32;
        let current_gap = gap(distance);
        if previous_gap >= 0.0 && current_gap <= 0.0 {
            let mut above = previous_distance;
            let mut below = distance;
            for _ in 0..14 {
                let middle = (above + below) * 0.5;
                if gap(middle) > 0.0 {
                    above = middle;
                } else {
                    below = middle;
                }
            }
            let mut point = ray.get_point((above + below) * 0.5);
            let uv = [
                ((point.x + half) / DEMO_TILE_SIZE).clamp(0.0, 1.0),
                ((point.z + half) / DEMO_TILE_SIZE).clamp(0.0, 1.0),
            ];
            point.y = tile.sample_bilinear(uv) * HEIGHT_DISPLAY_SCALE;
            return Some((point, uv));
        }
        previous_distance = distance;
        previous_gap = current_gap;
    }
    None
}

fn ray_axis_interval(
    origin: f32,
    direction: f32,
    minimum: f32,
    maximum: f32,
) -> Option<(f32, f32)> {
    if direction.abs() <= f32::EPSILON {
        return (origin >= minimum && origin <= maximum).then_some((0.0, f32::INFINITY));
    }
    let a = (minimum - origin) / direction;
    let b = (maximum - origin) / direction;
    Some((a.min(b), a.max(b)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coordinates::{CubeFace, TileId};

    #[test]
    fn terrain_pick_hits_displaced_height_instead_of_zero_plane() {
        let tile = HeightTile::flat(TileId::ROOT, 9, 1_000.0);
        let ray = Ray3d::new(Vec3::new(0.0, 10.0, 0.0), Dir3::NEG_Y);
        let (point, uv) = pick_terrain_surface(&ray, &tile).expect("ray should hit terrain");
        assert!((point.y - 2.5).abs() < 0.001);
        assert_eq!(uv, [0.5, 0.5]);
    }

    #[test]
    fn terrain_pick_rejects_rays_outside_the_tile() {
        let tile = HeightTile::flat(TileId::ROOT, 9, 0.0);
        let ray = Ray3d::new(Vec3::new(DEMO_TILE_SIZE, 10.0, 0.0), Dir3::NEG_Y);
        assert!(pick_terrain_surface(&ray, &tile).is_none());
    }

    #[test]
    fn local_tile_uv_maps_to_cube_face_coordinates() {
        let tile = TileId::new(CubeFace::NegativeZ, 2, 3, 1).unwrap();
        let position = surface_position_on_tile(tile, [0.5, 0.5], 120.0);
        assert_eq!(position.face, CubeFace::NegativeZ);
        assert_eq!([position.u, position.v], [0.75, -0.25]);
        assert_eq!(position.altitude_m, 120.0);
    }
}
