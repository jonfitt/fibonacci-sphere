//! Terrain type definitions and per-node terrain storage.

/// Side of sea level used when reassigning vertices in invalid terrain components.
///
/// Bands are derived from intrinsic elevation (e.g. Perlin sample sign) when
/// available, so post-processing can distinguish below-sea types (water, future
/// deep ocean) from above-sea types (land, mountain) without mixing them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ElevationBand {
    /// At or below sea level (`sample < 0` for noise-driven assigners).
    BelowSeaLevel,
    /// Above sea level (`sample >= 0` for noise-driven assigners).
    AboveSeaLevel,
}

impl ElevationBand {
    /// Classify a noise sample using the fixed sea level of `0.0`.
    pub fn from_sample(sample: f64) -> Self {
        if sample < 0.0 {
            Self::BelowSeaLevel
        } else {
            Self::AboveSeaLevel
        }
    }

    /// Terrain types that belong to this elevation band.
    pub fn terrain_types(self) -> &'static [TerrainType] {
        match self {
            Self::BelowSeaLevel => &[TerrainType::Water, TerrainType::DeepWater],
            Self::AboveSeaLevel => &[
                TerrainType::Land,
                TerrainType::Mountain,
                TerrainType::Ice,
                TerrainType::IceMountain,
            ],
        }
    }
}

/// Terrain classification for a lattice vertex.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerrainType {
    /// Dry land terrain.
    Land,
    /// Shallow water (below sea level, above the deep-water cutoff).
    Water,
    /// Deep water (most negative Perlin samples below sea level).
    DeepWater,
    /// Mountain terrain.
    Mountain,
    /// Polar lowland ice (above sea level, or below sea clamped to sea level in polar caps).
    Ice,
    /// Polar highland ice.
    IceMountain,
}

impl TerrainType {
    /// All terrain variants in a stable order.
    pub const ALL: [Self; 6] = [
        Self::Land,
        Self::Water,
        Self::DeepWater,
        Self::Mountain,
        Self::Ice,
        Self::IceMountain,
    ];

    /// Stable index for Godot and other FFI consumers (`Land`=0, `Water`=1, …).
    pub const fn godot_index(self) -> i32 {
        match self {
            Self::Land => 0,
            Self::Water => 1,
            Self::DeepWater => 2,
            Self::Mountain => 3,
            Self::Ice => 4,
            Self::IceMountain => 5,
        }
    }

    /// Parse a Godot/FFI terrain type index.
    pub fn from_godot_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(Self::Land),
            1 => Some(Self::Water),
            2 => Some(Self::DeepWater),
            3 => Some(Self::Mountain),
            4 => Some(Self::Ice),
            5 => Some(Self::IceMountain),
            _ => None,
        }
    }

    /// Elevation band for assigners that do not expose per-vertex noise samples.
    pub fn elevation_band(self) -> ElevationBand {
        match self {
            Self::Water | Self::DeepWater => ElevationBand::BelowSeaLevel,
            Self::Land | Self::Mountain | Self::Ice | Self::IceMountain => {
                ElevationBand::AboveSeaLevel
            }
        }
    }
}

/// Terrain assignment for every vertex in a surface graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainMap {
    terrain: Vec<TerrainType>,
}

impl TerrainMap {
    /// Build a terrain map with one entry per graph vertex.
    pub(crate) fn new(terrain: Vec<TerrainType>) -> Self {
        Self { terrain }
    }

    /// Number of vertices in the map.
    pub fn len(&self) -> usize {
        self.terrain.len()
    }

    /// Returns true when the map contains no vertices.
    pub fn is_empty(&self) -> bool {
        self.terrain.is_empty()
    }

    /// Terrain type at `index`.
    pub fn get(&self, index: usize) -> TerrainType {
        self.terrain[index]
    }

    /// All terrain assignments in vertex-index order.
    pub fn as_slice(&self) -> &[TerrainType] {
        &self.terrain
    }
}
