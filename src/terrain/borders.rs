//! Classification of Voronoi area borders for rendering and gameplay rules.

use super::areas::AreaKind;

/// Semantic category of a border between two adjacent terrain areas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AreaBorderKind {
    /// Same terrain class on both sides (unexpected for adjacent Voronoi cells).
    SameType,
    /// Coastline: above-sea terrain meets below-sea terrain.
    Coastline,
    /// Shallow water meets deep water.
    ShallowDeepWater,
    /// Land meets mountain.
    LandMountain,
}

impl AreaBorderKind {
    /// Stable index for Godot and other FFI consumers.
    pub const fn godot_index(self) -> i32 {
        match self {
            Self::SameType => 0,
            Self::Coastline => 1,
            Self::ShallowDeepWater => 2,
            Self::LandMountain => 3,
        }
    }

    /// Parse a Godot/FFI border kind index.
    pub fn from_godot_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(Self::SameType),
            1 => Some(Self::Coastline),
            2 => Some(Self::ShallowDeepWater),
            3 => Some(Self::LandMountain),
            _ => None,
        }
    }
}

/// Classify the border between two adjacent area kinds.
pub fn classify_area_border(left: AreaKind, right: AreaKind) -> AreaBorderKind {
    if left == right {
        return AreaBorderKind::SameType;
    }

    let left_above = is_above_sea(left);
    let right_above = is_above_sea(right);
    if left_above != right_above {
        return AreaBorderKind::Coastline;
    }

    if is_below_sea(left) && is_below_sea(right) {
        return AreaBorderKind::ShallowDeepWater;
    }

    AreaBorderKind::LandMountain
}

/// Returns true when the border crosses sea level (land/mountain vs water/deep water).
pub fn is_coastline_border(left: AreaKind, right: AreaKind) -> bool {
    classify_area_border(left, right) == AreaBorderKind::Coastline
}

fn is_above_sea(kind: AreaKind) -> bool {
    matches!(
        kind,
        AreaKind::Land | AreaKind::Mountain | AreaKind::Ice | AreaKind::IceMountain
    )
}

fn is_below_sea(kind: AreaKind) -> bool {
    matches!(kind, AreaKind::Water | AreaKind::DeepWater)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coastline_crosses_sea_level() {
        assert_eq!(
            classify_area_border(AreaKind::Land, AreaKind::Water),
            AreaBorderKind::Coastline
        );
        assert!(is_coastline_border(AreaKind::Mountain, AreaKind::DeepWater));
    }

    #[test]
    fn mountain_water_is_coastline() {
        assert_eq!(
            classify_area_border(AreaKind::Mountain, AreaKind::Water),
            AreaBorderKind::Coastline
        );
        assert_eq!(
            classify_area_border(AreaKind::Mountain, AreaKind::DeepWater),
            AreaBorderKind::Coastline
        );
    }

    #[test]
    fn shallow_deep_water_border() {
        assert_eq!(
            classify_area_border(AreaKind::Water, AreaKind::DeepWater),
            AreaBorderKind::ShallowDeepWater
        );
    }
}
