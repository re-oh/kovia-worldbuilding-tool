use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    coordinates::TileId,
    terrain::{ApplyHeightProposal, HeightProposal, HeightTile, TerrainStore},
};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct HydraulicErosionSettings {
    pub iterations: u32,
    pub rainfall: f32,
    pub flow_rate: f32,
    pub evaporation: f32,
    pub sediment_capacity: f32,
    pub erosion_rate: f32,
    pub deposition_rate: f32,
}

impl Default for HydraulicErosionSettings {
    fn default() -> Self {
        Self {
            iterations: 80,
            rainfall: 0.015,
            flow_rate: 0.55,
            evaporation: 0.04,
            sediment_capacity: 0.8,
            erosion_rate: 0.12,
            deposition_rate: 0.18,
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RunErosion {
    pub tile: TileId,
    pub settings: HydraulicErosionSettings,
}

#[derive(Debug, Clone, Copy, Event)]
pub struct ErosionFailed {
    pub tile: TileId,
}

/// CPU reference backend. It does not mutate canonical terrain: it emits the
/// same revision-guarded proposal contract a future WGSL backend will emit.
pub struct ErosionPlugin;

impl Plugin for ErosionPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(run_erosion);
    }
}

fn run_erosion(request: On<RunErosion>, terrain: Res<TerrainStore>, mut commands: Commands) {
    let Some(tile) = terrain.tiles.get(&request.tile) else {
        commands.trigger(ErosionFailed { tile: request.tile });
        return;
    };
    commands.trigger(ApplyHeightProposal(hydraulic_erosion_reference(
        tile,
        request.settings,
    )));
}

pub fn hydraulic_erosion_reference(
    tile: &HeightTile,
    settings: HydraulicErosionSettings,
) -> HeightProposal {
    let edge = tile.edge_samples as usize;
    let count = edge * edge;
    let mut height = tile.samples().to_vec();
    let mut water = vec![0.0_f32; count];
    let mut sediment = vec![0.0_f32; count];

    for _ in 0..settings.iterations {
        for value in &mut water {
            *value += settings.rainfall.max(0.0);
        }
        let mut water_delta = vec![0.0_f32; count];
        let mut sediment_delta = vec![0.0_f32; count];

        for y in 0..edge {
            for x in 0..edge {
                let index = y * edge + x;
                let surface = height[index] + water[index];
                let mut lowest_surface = surface;
                let mut target = None;
                for ny in y.saturating_sub(1)..=(y + 1).min(edge - 1) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(edge - 1) {
                        if nx == x && ny == y {
                            continue;
                        }
                        let neighbor = ny * edge + nx;
                        let neighbor_surface = height[neighbor] + water[neighbor];
                        if neighbor_surface < lowest_surface {
                            lowest_surface = neighbor_surface;
                            target = Some(neighbor);
                        }
                    }
                }
                let Some(target) = target else { continue };
                let drop = (surface - lowest_surface).max(0.0);
                let moved_water =
                    (water[index] * settings.flow_rate.clamp(0.0, 1.0)).min(drop * 0.5 + 0.001);
                if moved_water <= 0.0 {
                    continue;
                }
                let capacity = moved_water * drop * settings.sediment_capacity.max(0.0);
                if sediment[index] > capacity {
                    let deposited =
                        (sediment[index] - capacity) * settings.deposition_rate.clamp(0.0, 1.0);
                    height[index] += deposited;
                    sediment[index] -= deposited;
                } else {
                    let eroded = ((capacity - sediment[index])
                        * settings.erosion_rate.clamp(0.0, 1.0))
                    .min(4.0);
                    height[index] -= eroded;
                    sediment[index] += eroded;
                }
                let sediment_moved =
                    sediment[index] * (moved_water / water[index].max(0.000_001)).clamp(0.0, 1.0);
                water_delta[index] -= moved_water;
                water_delta[target] += moved_water;
                sediment_delta[index] -= sediment_moved;
                sediment_delta[target] += sediment_moved;
            }
        }
        for index in 0..count {
            water[index] = (water[index] + water_delta[index]).max(0.0)
                * (1.0 - settings.evaporation.clamp(0.0, 1.0));
            sediment[index] = (sediment[index] + sediment_delta[index]).max(0.0);
        }
    }
    HeightProposal {
        tile: tile.id,
        source_revision: tile.revision,
        samples: height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erosion_returns_revision_guarded_proposal() {
        let mut tile = HeightTile::flat(TileId::ROOT, 5, 0.0);
        tile.set(2, 2, 100.0);
        tile.touch();
        let proposal = hydraulic_erosion_reference(
            &tile,
            HydraulicErosionSettings {
                iterations: 4,
                ..Default::default()
            },
        );
        assert_eq!(proposal.source_revision, tile.revision);
        assert_ne!(proposal.samples, tile.samples());
    }
}
