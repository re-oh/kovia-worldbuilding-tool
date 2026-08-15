use std::collections::BTreeMap;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::coordinates::TileId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ResidencyState {
    Requested,
    CpuResident,
    GpuResident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidentTile {
    pub id: TileId,
    pub state: ResidencyState,
    pub last_used_frame: u64,
    pub cpu_bytes: u64,
    pub gpu_bytes: u64,
    /// Dirty tiles and tiles participating in active compute windows are pinned.
    pub pinned: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResidencyBudget {
    pub maximum_cpu_bytes: u64,
    pub maximum_gpu_bytes: u64,
    pub maximum_tiles: usize,
}

impl Default for ResidencyBudget {
    fn default() -> Self {
        Self {
            maximum_cpu_bytes: 384 * 1024 * 1024,
            maximum_gpu_bytes: 512 * 1024 * 1024,
            maximum_tiles: 192,
        }
    }
}

#[derive(Debug, Clone, Default, Resource, Serialize, Deserialize)]
pub struct TileResidencyManager {
    pub budget: ResidencyBudget,
    pub tiles: BTreeMap<TileId, ResidentTile>,
    pub frame: u64,
}

impl TileResidencyManager {
    pub fn begin_frame(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    pub fn request(&mut self, id: TileId) {
        self.tiles
            .entry(id)
            .and_modify(|tile| tile.last_used_frame = self.frame)
            .or_insert(ResidentTile {
                id,
                state: ResidencyState::Requested,
                last_used_frame: self.frame,
                cpu_bytes: 0,
                gpu_bytes: 0,
                pinned: false,
            });
    }

    pub fn mark_cpu_resident(&mut self, id: TileId, bytes: u64) {
        self.request(id);
        let tile = self.tiles.get_mut(&id).expect("tile was requested");
        tile.state = ResidencyState::CpuResident;
        tile.cpu_bytes = bytes;
    }

    pub fn mark_gpu_resident(&mut self, id: TileId, gpu_bytes: u64) {
        self.request(id);
        let tile = self.tiles.get_mut(&id).expect("tile was requested");
        tile.state = ResidencyState::GpuResident;
        tile.gpu_bytes = gpu_bytes;
    }

    pub fn touch(&mut self, id: TileId) {
        if let Some(tile) = self.tiles.get_mut(&id) {
            tile.last_used_frame = self.frame;
        }
    }

    pub fn set_pinned(&mut self, id: TileId, pinned: bool) {
        if let Some(tile) = self.tiles.get_mut(&id) {
            tile.pinned = pinned;
        }
    }

    pub fn usage(&self) -> (u64, u64, usize) {
        self.tiles
            .values()
            .fold((0, 0, 0), |(cpu, gpu, count), tile| {
                (cpu + tile.cpu_bytes, gpu + tile.gpu_bytes, count + 1)
            })
    }

    pub fn over_budget(&self) -> bool {
        let (cpu, gpu, count) = self.usage();
        cpu > self.budget.maximum_cpu_bytes
            || gpu > self.budget.maximum_gpu_bytes
            || count > self.budget.maximum_tiles
    }

    /// Returns oldest unpinned tiles. The caller performs save/upload teardown
    /// and removes entries only after those operations succeed.
    pub fn eviction_candidates(&self) -> Vec<TileId> {
        let mut candidates: Vec<_> = self
            .tiles
            .values()
            .filter(|tile| !tile.pinned)
            .map(|tile| (tile.last_used_frame, tile.id))
            .collect();
        candidates.sort_unstable();
        candidates.into_iter().map(|(_, id)| id).collect()
    }
}

/// Memory policy only. Loading, persistence, and GPU upload plugins may use
/// this service without being pulled into a monolithic streaming subsystem.
pub struct TileResidencyPlugin;

impl Plugin for TileResidencyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TileResidencyManager>()
            .add_systems(First, begin_residency_frame);
    }
}

fn begin_residency_frame(mut residency: ResMut<TileResidencyManager>) {
    residency.begin_frame();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_tiles_are_not_eviction_candidates() {
        let mut manager = TileResidencyManager::default();
        let root = TileId::ROOT;
        let child = root.children()[0];
        manager.request(root);
        manager.begin_frame();
        manager.request(child);
        manager.set_pinned(root, true);
        assert_eq!(manager.eviction_candidates(), vec![child]);
    }
}
