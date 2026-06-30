//! Literature-backed descriptions of each distribution method.

use super::OptimizationGoal;

/// Purpose, trade-offs, and references for a distribution method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MethodInfo {
    /// Short label for UI and logs.
    pub display_name: &'static str,
    /// One sentence on what the method is for.
    pub purpose: &'static str,
    /// Primary strengths (each item is one bullet).
    pub advantages: &'static [&'static str],
    /// Known limitations (each item is one bullet).
    pub disadvantages: &'static [&'static str],
    /// Suggested reading; URLs or citation strings.
    pub references: &'static [&'static str],
}

impl MethodInfo {
    /// Multi-line description for HUDs and tooltips (includes goal line).
    pub fn format_description(&self, goal: OptimizationGoal) -> String {
        let mut out = String::new();
        out.push_str(self.display_name);
        out.push_str("\nGoal: ");
        out.push_str(&format!("{goal:?}"));
        out.push_str("\n\n");
        out.push_str(self.purpose);
        out.push_str("\n\nAdvantages:\n");
        for item in self.advantages {
            out.push_str("  • ");
            out.push_str(item);
            out.push('\n');
        }
        out.push_str("\nDisadvantages:\n");
        for item in self.disadvantages {
            out.push_str("  • ");
            out.push_str(item);
            out.push('\n');
        }
        out.push_str("\nReferences:\n");
        for reference in self.references {
            out.push_str("  • ");
            out.push_str(reference);
            out.push('\n');
        }
        out
    }
}

/// Metadata for [`super::DistributionMethod::CanonicalMidpoint`].
pub const CANONICAL_MIDPOINT: MethodInfo = MethodInfo {
    display_name: "Canonical Midpoint",
    purpose: "Default Fibonacci (golden-angle) spiral with a half-step colatitude offset \
              `(i + 0.5) / n`. Maps an equal-area vertical parameter to the sphere using \
              azimuth `2π i / φ`. Intended as a fast, deterministic quasi-uniform sampler \
              with better pole behavior than the raw canonical lattice.",
    advantages: &[
        "O(n) closed form — no iteration or rejection (Keinert et al., spherical Fibonacci)",
        "Low-discrepancy golden-angle azimuth gives visually even coverage",
        "Midpoint colatitude avoids placing the first sample exactly at the north pole",
        "Good general-purpose compromise between speed and minimum neighbor distance",
    ],
    disadvantages: &[
        "Not a true equal-area point partition; Voronoi cells vary in area",
        "Does not maximize minimum separation (Tammes / Thomson problems are harder)",
        "No samples at exact poles unless n and mapping align — polar cap can look sparse",
        "Neighbor distance varies more than optimized offset lattices (Roberts 2018)",
    ],
    references: &[
        "Keinert et al., \"Spherical Fibonacci Mapping\" (ACM TOG 2015)",
        "Gonzalez, \"Measurement of areas on a sphere using Fibonacci and latitude-longitude \
         lattices\" (arXiv:0912.4540)",
        "Roberts, \"How to evenly distribute points on a sphere…\" (Extreme Learning, 2018)",
    ],
};

/// Metadata for [`super::DistributionMethod::Canonical`].
pub const CANONICAL: MethodInfo = MethodInfo {
    display_name: "Canonical",
    purpose: "Baseline Fibonacci lattice with colatitude `(i + 0) / n` and golden-angle \
              azimuth. Serves as the unmodified reference against which midpoint and offset \
              variants are compared.",
    advantages: &[
        "Simplest Fibonacci sphere formula — easy to reproduce in papers and shaders",
        "Deterministic, O(n), and well documented in graphics and MRI trajectory literature",
        "First sample lies on the north pole — useful when a pole anchor is desired without \
         a separate pole method",
    ],
    disadvantages: &[
        "Strongest polar clustering of the Fibonacci family; minimum neighbor distance is \
         often limited by points near the poles (Roberts 2018)",
        "Worse packing than midpoint or offset lattices at the same n",
        "Equal-area in the continuous limit but uneven discrete Voronoi areas at finite n",
    ],
    references: &[
        "Saff & Kuijlaars, distributing points on the sphere (Math. Intelligencer 1997)",
        "Roberts, \"Evenly distributing points on a sphere\" (Extreme Learning, 2018)",
        "Baskerville, \"On the Use of Fibonacci Lattices for Spherical Point Sets\" (2024)",
    ],
};

/// Metadata for [`super::DistributionMethod::OffsetPacking`].
pub const OFFSET_PACKING: MethodInfo = MethodInfo {
    display_name: "Offset Packing",
    purpose: "Roberts-style offset Fibonacci lattice: colatitude uses `(i + ε) / (n - 1 + 2ε)` \
              with tiered ε chosen to tighten minimum nearest-neighbor distance. Pulls samples \
              slightly away from the poles where the canonical lattice is most crowded.",
    advantages: &[
        "Targets minimum packing distance δ_min — up to ~8% improvement over canonical \
         Fibonacci (Roberts 2018)",
        "Adaptive ε table tracks optimal offset as n grows",
        "Same O(n) cost as canonical; drop-in for real-time sampling",
        "Better worst-case neighbor gaps for collision, LOD, and meshing",
    ],
    disadvantages: &[
        "Requires ε lookup — magic constants tied to empirical optimization, not a closed form",
        "Optimizes min distance, not variance; average spacing can be less uniform than \
         Offset Average Neighbor",
        "Still leaves a small polar cap void (no sample at exact poles)",
        "Quality is heuristic; not a provably optimal spherical t-design",
    ],
    references: &[
        "Roberts, \"How to evenly distribute points on a sphere more effectively…\" (2018)",
        "Roberts, \"Evenly distributing points on a sphere\" — lattice #2 and #3 (2018)",
    ],
};

/// Metadata for [`super::DistributionMethod::OffsetPackingWithPoles`].
pub const OFFSET_PACKING_WITH_POLES: MethodInfo = MethodInfo {
    display_name: "Offset Packing With Poles",
    purpose: "Places explicit north and south pole samples, then distributes the remaining \
              `n − 2` points with a pole-aware offset lattice (Roberts \"lattice with poles\"). \
              Removes the polar void while keeping improved packing in the mid-latitudes.",
    advantages: &[
        "Guaranteed axis samples at ±Y — useful for planets, joints, and Y-up engines",
        "Closes the polar Voronoi gap that offset packing alone leaves (Roberts 2018)",
        "For large n, can exceed plain offset packing on combined hull / Voronoi metrics",
        "Deterministic and fast like other Fibonacci variants",
    ],
    disadvantages: &[
        "Two samples are fixed at poles — reduces freedom for mid-latitude packing when n is small",
        "Pole neighbors are forced; local spacing at caps can still be uneven for tiny n",
        "Separate ε table from non-pole offset packing; must keep tables in sync when tuning",
        "Not ideal if you must avoid singularities at poles (e.g. equirectangular seams)",
    ],
    references: &[
        "Roberts, \"Evenly distributing points on a sphere\" — equation (4), lattice with poles",
        "Roberts, \"How to evenly distribute points on a sphere more effectively…\" (2018)",
    ],
};

/// Metadata for [`super::DistributionMethod::OffsetAverageNeighbor`].
pub const OFFSET_AVERAGE_NEIGHBOR: MethodInfo = MethodInfo {
    display_name: "Offset Average Neighbor",
    purpose: "Offset Fibonacci lattice with fixed ε ≈ 0.36, tuned for more uniform \
              nearest-neighbor distances overall rather than maximizing only the minimum gap. \
              Favors consistent local spacing over worst-case packing.",
    advantages: &[
        "Reduces max/min neighbor ratio compared with canonical lattice (~10–15% in practice; \
         Baskerville 2024 citing Roberts)",
        "Single constant ε — no tier table, simpler than Offset Packing",
        "Useful when visual or physical uniformity matters more than absolute min distance",
    ],
    disadvantages: &[
        "Does not maximize δ_min as aggressively as tiered Offset Packing",
        "Fixed ε is a compromise across all n — not re-optimized per count",
        "Still no guaranteed pole samples",
        "Empirically tuned; less documented than the tiered packing table",
    ],
    references: &[
        "Roberts (2018), offset model with ε = 0.36 for average neighbor uniformity",
        "Baskerville, \"On the Use of Fibonacci Lattices for Spherical Point Sets\" (2024)",
    ],
};

/// Metadata for [`super::DistributionMethod::LatitudeLongitude`].
pub const LATITUDE_LONGITUDE: MethodInfo = MethodInfo {
    display_name: "Latitude–Longitude",
    purpose: "Regular lat–long grid: equal-area colatitude rings with longitude samples \
              per ring. Included as a baseline for quadrature, climate-style data, and \
              comparisons to Fibonacci lattices (Gonzalez 2009).",
    advantages: &[
        "Structured rings align with geographic coordinates and equirectangular textures",
        "Equal-area ring spacing in colatitude (continuous limit)",
        "Predictable indexing — easy to map to UV spheres and lat–long datasets",
        "Useful reference when comparing Fibonacci methods to classical grids",
    ],
    disadvantages: &[
        "Severe point clustering toward the poles at equal ring counts (Gonzalez 2009)",
        "Large variation in nearest-neighbor distance across the sphere",
        "Poor packing and Delaunay mesh quality near caps compared with Fibonacci lattices",
        "Not optimal for Monte Carlo integration or unbiased surface sampling",
    ],
    references: &[
        "Gonzalez, \"Measurement of areas on a sphere using Fibonacci and latitude-longitude \
         lattices\" (arXiv:0912.4540)",
        "Standard geospatial / equirectangular sampling literature",
    ],
};

#[cfg(test)]
mod tests {
    use crate::DistributionMethod;

    #[test]
    fn every_method_has_nonempty_info() {
        for method in DistributionMethod::ALL {
            let info = method.info();
            assert!(!info.display_name.is_empty());
            assert!(!info.purpose.is_empty());
            assert!(!info.advantages.is_empty());
            assert!(!info.disadvantages.is_empty());
            assert!(!info.references.is_empty());
        }
    }
}
