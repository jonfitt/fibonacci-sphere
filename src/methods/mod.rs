mod canonical;
mod distribution;
mod epsilon;
mod info;
mod latlong;
mod offset;

pub use canonical::{Canonical, CanonicalMidpoint};
pub use distribution::{Distribution, OptimizationGoal};
pub use info::MethodInfo;
pub use latlong::LatitudeLongitude;
pub use offset::{OffsetAverageNeighbor, OffsetPacking, OffsetPackingWithPoles};

use crate::error::SphereError;
use crate::validation::validate_lattice_params;

/// Selectable distribution method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DistributionMethod {
    /// Midpoint canonical Fibonacci lattice; good general-purpose packing.
    ///
    /// See [`info::CANONICAL_MIDPOINT`] for purpose, trade-offs, and references.
    #[default]
    CanonicalMidpoint,
    /// Original Fibonacci lattice without midpoint offset.
    ///
    /// See [`info::CANONICAL`] for purpose, trade-offs, and references.
    Canonical,
    /// Offset lattice optimized for minimum nearest-neighbor distance.
    ///
    /// Uses tiered ε values from Roberts (2018). See [`info::OFFSET_PACKING`].
    OffsetPacking,
    /// Offset lattice with explicit north and south pole points.
    ///
    /// See [`info::OFFSET_PACKING_WITH_POLES`].
    OffsetPackingWithPoles,
    /// Offset lattice optimized for average nearest-neighbor distance.
    ///
    /// Fixed ε ≈ 0.36. See [`info::OFFSET_AVERAGE_NEIGHBOR`].
    OffsetAverageNeighbor,
    /// Regular latitude-longitude grid baseline.
    ///
    /// See [`info::LATITUDE_LONGITUDE`].
    LatitudeLongitude,
}

impl DistributionMethod {
    /// All available methods.
    pub const ALL: [Self; 6] = [
        Self::CanonicalMidpoint,
        Self::Canonical,
        Self::OffsetPacking,
        Self::OffsetPackingWithPoles,
        Self::OffsetAverageNeighbor,
        Self::LatitudeLongitude,
    ];

    /// Human-readable method name.
    pub fn name(self) -> &'static str {
        self.distribution().name()
    }

    /// Primary optimization goal.
    pub fn optimizes(self) -> OptimizationGoal {
        self.distribution().optimizes()
    }

    /// Literature-backed purpose, advantages, disadvantages, and references.
    pub fn info(self) -> &'static MethodInfo {
        match self {
            Self::CanonicalMidpoint => &info::CANONICAL_MIDPOINT,
            Self::Canonical => &info::CANONICAL,
            Self::OffsetPacking => &info::OFFSET_PACKING,
            Self::OffsetPackingWithPoles => &info::OFFSET_PACKING_WITH_POLES,
            Self::OffsetAverageNeighbor => &info::OFFSET_AVERAGE_NEIGHBOR,
            Self::LatitudeLongitude => &info::LATITUDE_LONGITUDE,
        }
    }

    /// Multi-line HUD description for this method.
    pub fn format_description(self) -> String {
        self.info().format_description(self.optimizes())
    }

    fn distribution(self) -> &'static dyn Distribution {
        match self {
            Self::Canonical => &Canonical,
            Self::CanonicalMidpoint => &CanonicalMidpoint,
            Self::OffsetPacking => &OffsetPacking,
            Self::OffsetPackingWithPoles => &OffsetPackingWithPoles,
            Self::OffsetAverageNeighbor => &OffsetAverageNeighbor,
            Self::LatitudeLongitude => &LatitudeLongitude,
        }
    }

    /// Generate points using this method.
    pub fn generate(
        self,
        n: usize,
        radius: f64,
    ) -> Result<Vec<crate::point::SpherePoint>, SphereError> {
        validate_lattice_params(n, radius)?;
        Ok(self.distribution().generate(n, radius))
    }

    /// Godot-facing index for this method (see `docs/godot.md`).
    pub fn to_godot_index(self) -> i32 {
        match self {
            Self::CanonicalMidpoint => 0,
            Self::Canonical => 1,
            Self::OffsetPacking => 2,
            Self::OffsetPackingWithPoles => 3,
            Self::OffsetAverageNeighbor => 4,
            Self::LatitudeLongitude => 5,
        }
    }

    /// Resolve a Godot method index to a distribution method.
    pub fn from_godot_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(Self::CanonicalMidpoint),
            1 => Some(Self::Canonical),
            2 => Some(Self::OffsetPacking),
            3 => Some(Self::OffsetPackingWithPoles),
            4 => Some(Self::OffsetAverageNeighbor),
            5 => Some(Self::LatitudeLongitude),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_on_sphere, assert_sequential_indices};

    #[test]
    fn godot_index_roundtrip() {
        for method in DistributionMethod::ALL {
            let index = method.to_godot_index();
            assert_eq!(DistributionMethod::from_godot_index(index), Some(method));
        }
        assert!(DistributionMethod::from_godot_index(-1).is_none());
        assert!(DistributionMethod::from_godot_index(6).is_none());
    }

    #[test]
    fn default_method_is_canonical_midpoint() {
        assert_eq!(
            DistributionMethod::default(),
            DistributionMethod::CanonicalMidpoint
        );
    }

    #[test]
    fn all_lists_every_variant_once() {
        assert_eq!(DistributionMethod::ALL.len(), 6);
        let mut sorted = DistributionMethod::ALL;
        sorted.sort_by_key(|m| format!("{m:?}"));
        assert_eq!(sorted[0], DistributionMethod::Canonical);
        assert_eq!(sorted[5], DistributionMethod::OffsetPackingWithPoles);
    }

    #[test]
    fn names_are_non_empty_and_distinct() {
        let names: Vec<_> = DistributionMethod::ALL.iter().map(|m| m.name()).collect();
        assert!(names.iter().all(|n| !n.is_empty()));
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len());
    }

    #[test]
    fn optimization_goals_match_method() {
        assert_eq!(
            DistributionMethod::Canonical.optimizes(),
            OptimizationGoal::Baseline
        );
        assert_eq!(
            DistributionMethod::CanonicalMidpoint.optimizes(),
            OptimizationGoal::PackingDistance
        );
        assert_eq!(
            DistributionMethod::OffsetPacking.optimizes(),
            OptimizationGoal::PackingDistance
        );
        assert_eq!(
            DistributionMethod::OffsetPackingWithPoles.optimizes(),
            OptimizationGoal::PackingDistance
        );
        assert_eq!(
            DistributionMethod::OffsetAverageNeighbor.optimizes(),
            OptimizationGoal::AverageNeighborDistance
        );
        assert_eq!(
            DistributionMethod::LatitudeLongitude.optimizes(),
            OptimizationGoal::EqualArea
        );
    }

    #[test]
    fn rejects_zero_point_count() {
        assert_eq!(
            DistributionMethod::CanonicalMidpoint.generate(0, 1.0),
            Err(SphereError::InvalidPointCount { n: 0 })
        );
    }

    #[test]
    fn rejects_non_positive_radius() {
        assert_eq!(
            DistributionMethod::Canonical.generate(10, 0.0),
            Err(SphereError::InvalidRadius { radius: 0.0 })
        );
        assert_eq!(
            DistributionMethod::Canonical.generate(10, -1.0),
            Err(SphereError::InvalidRadius { radius: -1.0 })
        );
    }

    #[test]
    fn generate_delegates_to_distribution() {
        for method in DistributionMethod::ALL {
            let points = method.generate(12, 1.5).unwrap();
            assert_eq!(points.len(), 12);
            assert_on_sphere(&points, 1.5);
            assert_sequential_indices(&points);
        }
    }
}
