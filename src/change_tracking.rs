use std::collections::BTreeMap;

use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::{
    coordinates::TileId, features::FeatureChanged, layers::RegionChanged, terrain::TerrainChanged,
};

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
    pub struct DerivedProducts: u16 {
        const NORMALS = 1 << 0;
        const SLOPE = 1 << 1;
        const FLOW = 1 << 2;
        const EROSION = 1 << 3;
        const RIVERS = 1 << 4;
        const MOISTURE = 1 << 5;
        const BIOMES = 1 << 6;
        const REGION_BOUNDARIES = 1 << 7;
        const LABEL_LAYOUT = 1 << 8;
    }
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct DirtyTracker {
    pub tiles: BTreeMap<TileId, DerivedProducts>,
    pub feature_labels: bool,
}

impl DirtyTracker {
    pub fn mark(&mut self, tile: TileId, products: DerivedProducts) {
        self.tiles
            .entry(tile)
            .and_modify(|dirty| *dirty |= products)
            .or_insert(products);
    }

    pub fn clear(&mut self, tile: TileId, products: DerivedProducts) {
        if let Some(dirty) = self.tiles.get_mut(&tile) {
            dirty.remove(products);
            if dirty.is_empty() {
                self.tiles.remove(&tile);
            }
        }
    }
}

/// Reacts to public change events. It owns no terrain, layer, or feature data.
pub struct ChangeTrackingPlugin;

impl Plugin for ChangeTrackingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DirtyTracker>()
            .add_observer(invalidate_from_height)
            .add_observer(invalidate_from_regions)
            .add_observer(invalidate_from_features);
    }
}

fn invalidate_from_height(change: On<TerrainChanged>, mut dirty: ResMut<DirtyTracker>) {
    dirty.mark(
        change.tile,
        DerivedProducts::NORMALS
            | DerivedProducts::SLOPE
            | DerivedProducts::FLOW
            | DerivedProducts::EROSION
            | DerivedProducts::RIVERS
            | DerivedProducts::MOISTURE
            | DerivedProducts::BIOMES,
    );
}

fn invalidate_from_regions(change: On<RegionChanged>, mut dirty: ResMut<DirtyTracker>) {
    dirty.mark(
        change.tile,
        DerivedProducts::REGION_BOUNDARIES | DerivedProducts::LABEL_LAYOUT,
    );
}

fn invalidate_from_features(_change: On<FeatureChanged>, mut dirty: ResMut<DirtyTracker>) {
    dirty.feature_labels = true;
}
