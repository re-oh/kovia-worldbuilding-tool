use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};

use crate::{
    editor::{ActiveEditTile, LayerSelection},
    layers::{LayerDefinition, LayerRegistry, LayerStore, RegionTile},
    persistence::ProjectLoaded,
    terrain::{HeightTile, TerrainStore},
};

pub const DEMO_TILE_SIZE: f32 = 20.0;
pub const HEIGHT_DISPLAY_SCALE: f32 = 0.0025;

#[derive(Debug, Component)]
pub struct RenderedTerrainTile {
    pub id: crate::coordinates::TileId,
    pub terrain_revision: u64,
    pub overlay_signature: u64,
}

/// Disposable visualization cache. It can render terrain without layer
/// resources; a present selected layer is an optional overlay input.
pub struct TerrainRenderPlugin;

impl Plugin for TerrainRenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (spawn_active_tile, sync_terrain_mesh).chain());
        app.add_observer(clear_render_cache_after_load);
    }
}

fn clear_render_cache_after_load(
    _event: On<ProjectLoaded>,
    rendered: Query<Entity, With<RenderedTerrainTile>>,
    mut commands: Commands,
) {
    for entity in &rendered {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_active_tile(
    mut commands: Commands,
    active: Res<ActiveEditTile>,
    terrain: Res<TerrainStore>,
    registry: Option<Res<LayerRegistry>>,
    layers: Option<Res<LayerStore>>,
    selection: Option<Res<LayerSelection>>,
    rendered: Query<&RenderedTerrainTile>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if rendered.iter().any(|rendered| rendered.id == active.0) {
        return;
    }
    let Some(tile) = terrain.tiles.get(&active.0) else {
        return;
    };
    let overlay = selected_region_overlay(
        registry.as_deref(),
        layers.as_deref(),
        selection.as_deref(),
        active.0,
    );
    let signature = overlay_signature(selection.as_deref().and_then(|s| s.layer), overlay);
    let mesh = meshes.add(build_height_mesh(tile, overlay));
    let material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        perceptual_roughness: 0.92,
        metallic: 0.0,
        ..default()
    });
    commands.spawn((
        Name::new("Terrain tile render cache"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        RenderedTerrainTile {
            id: active.0,
            terrain_revision: tile.revision,
            overlay_signature: signature,
        },
    ));
}

fn sync_terrain_mesh(
    terrain: Res<TerrainStore>,
    registry: Option<Res<LayerRegistry>>,
    layers: Option<Res<LayerStore>>,
    selection: Option<Res<LayerSelection>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut rendered: Query<(&Mesh3d, &mut RenderedTerrainTile)>,
) {
    for (mesh_handle, mut rendered_tile) in &mut rendered {
        let Some(tile) = terrain.tiles.get(&rendered_tile.id) else {
            continue;
        };
        let overlay = selected_region_overlay(
            registry.as_deref(),
            layers.as_deref(),
            selection.as_deref(),
            rendered_tile.id,
        );
        let signature = overlay_signature(selection.as_deref().and_then(|s| s.layer), overlay);
        if rendered_tile.terrain_revision == tile.revision
            && rendered_tile.overlay_signature == signature
        {
            continue;
        }
        if let Some(mut mesh) = meshes.get_mut(&mesh_handle.0) {
            *mesh = build_height_mesh(tile, overlay);
            rendered_tile.terrain_revision = tile.revision;
            rendered_tile.overlay_signature = signature;
        }
    }
}

fn selected_region_overlay<'a>(
    registry: Option<&'a LayerRegistry>,
    layers: Option<&'a LayerStore>,
    selection: Option<&LayerSelection>,
    tile: crate::coordinates::TileId,
) -> Option<(&'a RegionTile, &'a LayerDefinition)> {
    let layer = selection?.layer?;
    let definition = registry?.definitions.get(&layer)?;
    if !definition.visible {
        return None;
    }
    Some((layers?.categorical_tiles.get(&(layer, tile))?, definition))
}

fn overlay_signature(
    layer: Option<crate::layers::LayerId>,
    overlay: Option<(&RegionTile, &LayerDefinition)>,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    layer.hash(&mut hasher);
    if let Some((tile, definition)) = overlay {
        tile.revision.hash(&mut hasher);
        definition.visible.hash(&mut hasher);
        definition.opacity.to_bits().hash(&mut hasher);
        for (code, region) in &definition.regions {
            code.hash(&mut hasher);
            region.color_rgba.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn build_height_mesh(tile: &HeightTile, overlay: Option<(&RegionTile, &LayerDefinition)>) -> Mesh {
    let edge = tile.edge_samples as usize;
    let cell_world = DEMO_TILE_SIZE / (edge - 1) as f32;
    let mut positions = Vec::with_capacity(edge * edge);
    let mut normals = Vec::with_capacity(edge * edge);
    let mut uvs = Vec::with_capacity(edge * edge);
    let mut colors = Vec::with_capacity(edge * edge);

    for y in 0..edge {
        for x in 0..edge {
            let u = x as f32 / (edge - 1) as f32;
            let v = y as f32 / (edge - 1) as f32;
            let height_m = tile.get(x, y);
            positions.push([
                (u - 0.5) * DEMO_TILE_SIZE,
                height_m * HEIGHT_DISPLAY_SCALE,
                (v - 0.5) * DEMO_TILE_SIZE,
            ]);
            uvs.push([u, v]);
            let left = tile.get(x.saturating_sub(1), y);
            let right = tile.get((x + 1).min(edge - 1), y);
            let down = tile.get(x, y.saturating_sub(1));
            let up = tile.get(x, (y + 1).min(edge - 1));
            let dx = (right - left) * HEIGHT_DISPLAY_SCALE / (2.0 * cell_world);
            let dz = (up - down) * HEIGHT_DISPLAY_SCALE / (2.0 * cell_world);
            normals.push(Vec3::new(-dx, 1.0, -dz).normalize_or_zero().to_array());

            let mut color = terrain_color(height_m);
            if let Some((region_tile, definition)) = overlay {
                let cx = x.min(region_tile.edge_cells as usize - 1);
                let cy = y.min(region_tile.edge_cells as usize - 1);
                let code = region_tile.cells()[region_tile.index(cx, cy)];
                if let Some(region) = definition.regions.get(&code) {
                    let tint = [
                        region.color_rgba[0] as f32 / 255.0,
                        region.color_rgba[1] as f32 / 255.0,
                        region.color_rgba[2] as f32 / 255.0,
                        1.0,
                    ];
                    color = mix_rgba(color, tint, definition.opacity * 0.58);
                }
            }
            colors.push(color);
        }
    }
    let mut indices = Vec::with_capacity((edge - 1) * (edge - 1) * 6);
    for y in 0..edge - 1 {
        for x in 0..edge - 1 {
            let i = (y * edge + x) as u32;
            let below = i + edge as u32;
            indices.extend_from_slice(&[i, below, i + 1, i + 1, below, below + 1]);
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, colors)
    .with_inserted_indices(Indices::U32(indices))
}

fn terrain_color(height_m: f32) -> [f32; 4] {
    if height_m < 0.0 {
        let depth = (-height_m / 350.0).clamp(0.0, 1.0);
        mix_rgba([0.10, 0.34, 0.46, 1.0], [0.025, 0.10, 0.20, 1.0], depth)
    } else if height_m < 900.0 {
        let t = (height_m / 900.0).clamp(0.0, 1.0);
        mix_rgba([0.29, 0.43, 0.24, 1.0], [0.43, 0.36, 0.25, 1.0], t)
    } else {
        let t = ((height_m - 900.0) / 1_200.0).clamp(0.0, 1.0);
        mix_rgba([0.43, 0.36, 0.25, 1.0], [0.84, 0.85, 0.82, 1.0], t)
    }
}

fn mix_rgba(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}
