use serde::{Deserialize, Serialize};

/// Shared spatial brush vocabulary. It has no Bevy, terrain, layer, UI, or
/// renderer dependency.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum BrushFalloff {
    Constant,
    Linear,
    Smooth,
}

impl BrushFalloff {
    pub fn weight(self, normalized_distance: f32) -> f32 {
        let t = (1.0 - normalized_distance).clamp(0.0, 1.0);
        match self {
            Self::Constant => (normalized_distance <= 1.0) as u8 as f32,
            Self::Linear => t,
            Self::Smooth => t * t * (3.0 - 2.0 * t),
        }
    }
}
