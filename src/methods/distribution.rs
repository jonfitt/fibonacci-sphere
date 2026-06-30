use crate::point::SpherePoint;

/// What a distribution method primarily optimizes for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationGoal {
    /// Baseline reference; no specific optimization (canonical Fibonacci).
    Baseline,
    /// Minimum nearest-neighbor (packing) distance — related to Tammes / hard-sphere bounds.
    PackingDistance,
    /// Uniformity of nearest-neighbor distances (lower max/min ratio).
    AverageNeighborDistance,
    /// Equal-area ring structure; not equal point density (lat–long baseline).
    EqualArea,
}

/// Algorithm for distributing `n` points on a sphere.
pub trait Distribution {
    /// Human-readable method name.
    fn name(&self) -> &'static str;

    /// Primary optimization goal of this method.
    fn optimizes(&self) -> OptimizationGoal;

    /// Generate `n` points on a sphere of the given radius.
    fn generate(&self, n: usize, radius: f64) -> Vec<SpherePoint>;
}
