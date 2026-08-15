use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{
    change_tracking::{DerivedProducts, DirtyTracker},
    coordinates::TileId,
    terrain::{HeightTile, TerrainChanged, TerrainStore},
};

#[derive(Debug, Clone, PartialEq)]
pub struct HydrologyResult {
    pub edge_samples: u16,
    pub receiver: Vec<Option<u32>>,
    pub accumulation: Vec<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiverSegment {
    pub from: u32,
    pub to: u32,
    pub discharge: f32,
}

#[derive(Debug, Clone)]
pub struct HydrologyTile {
    pub source_revision: u64,
    pub edge_samples: u16,
    pub segments: Vec<RiverSegment>,
}

#[derive(Debug, Default, Resource)]
pub struct HydrologyCache {
    pub tiles: BTreeMap<TileId, HydrologyTile>,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct ComputeHydrology {
    pub tile: TileId,
    pub rainfall: f32,
    pub river_threshold: f32,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct HydrologyUpdated {
    pub tile: TileId,
    pub river_segments: usize,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct HydrologyFailed {
    pub tile: TileId,
}

/// Reference hydrology backend. It is independently replaceable by a task-pool
/// or GPU backend because its only public product is `HydrologyCache`.
pub struct HydrologyPlugin;

impl Plugin for HydrologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HydrologyCache>()
            .add_observer(compute_hydrology)
            .add_observer(invalidate_stale_hydrology);
    }
}

fn compute_hydrology(
    request: On<ComputeHydrology>,
    terrain: Res<TerrainStore>,
    mut cache: ResMut<HydrologyCache>,
    mut dirty: Option<ResMut<DirtyTracker>>,
    mut commands: Commands,
) {
    let Some(tile) = terrain.tiles.get(&request.tile) else {
        commands.trigger(HydrologyFailed { tile: request.tile });
        return;
    };
    let flow = compute_d8_flow(tile, request.rainfall);
    let segments = extract_river_segments(&flow, request.river_threshold);
    let count = segments.len();
    cache.tiles.insert(
        request.tile,
        HydrologyTile {
            source_revision: tile.revision,
            edge_samples: flow.edge_samples,
            segments,
        },
    );
    if let Some(dirty) = dirty.as_deref_mut() {
        dirty.clear(
            request.tile,
            DerivedProducts::FLOW | DerivedProducts::RIVERS,
        );
    }
    commands.trigger(HydrologyUpdated {
        tile: request.tile,
        river_segments: count,
    });
}

fn invalidate_stale_hydrology(change: On<TerrainChanged>, mut cache: ResMut<HydrologyCache>) {
    cache.tiles.remove(&change.tile);
}

pub fn compute_d8_flow(tile: &HeightTile, rainfall: f32) -> HydrologyResult {
    let edge = tile.edge_samples as usize;
    let count = edge * edge;
    let mut receiver = vec![None; count];

    for y in 0..edge {
        for x in 0..edge {
            let index = y * edge + x;
            let height = tile.samples()[index];
            let mut lowest = height;
            let mut target = None;
            for ny in y.saturating_sub(1)..=(y + 1).min(edge - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(edge - 1) {
                    if nx == x && ny == y {
                        continue;
                    }
                    let neighbor = ny * edge + nx;
                    let neighbor_height = tile.samples()[neighbor];
                    if neighbor_height < lowest {
                        lowest = neighbor_height;
                        target = Some(neighbor as u32);
                    }
                }
            }
            receiver[index] = target;
        }
    }

    let mut order: Vec<usize> = (0..count).collect();
    order.sort_unstable_by(|a, b| tile.samples()[*b].total_cmp(&tile.samples()[*a]));
    let mut accumulation = vec![rainfall.max(0.0); count];
    for index in order {
        if let Some(target) = receiver[index] {
            accumulation[target as usize] += accumulation[index];
        }
    }
    HydrologyResult {
        edge_samples: tile.edge_samples,
        receiver,
        accumulation,
    }
}

pub fn extract_river_segments(flow: &HydrologyResult, threshold: f32) -> Vec<RiverSegment> {
    flow.receiver
        .iter()
        .enumerate()
        .filter_map(|(from, to)| {
            let discharge = flow.accumulation[from];
            let to = (*to)?;
            (discharge >= threshold).then_some(RiverSegment {
                from: from as u32,
                to,
                discharge,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::HeightTile;

    #[test]
    fn drainage_accumulates_downhill() {
        let mut tile = HeightTile::flat(TileId::ROOT, 5, 0.0);
        for y in 0..5 {
            for x in 0..5 {
                tile.set(x, y, (x + y) as f32);
            }
        }
        tile.touch();
        let flow = compute_d8_flow(&tile, 1.0);
        assert!(flow.accumulation[0] > 1.0);
        assert!(!extract_river_segments(&flow, 3.0).is_empty());
    }
}
