//! Tiered ε lookup tables for Roberts-style offset lattices.

/// Lookup epsilon for the offset packing lattice (minimum nearest-neighbor distance).
///
/// Tiered values from Martin Roberts, *How to evenly distribute points on a sphere more
/// effectively than the canonical Fibonacci lattice* (Extreme Learning, 2018). Each tier
/// re-optimizes ε for the pole-limited packing distance at that sample count.
pub fn packing_epsilon(n: usize) -> f64 {
    match n {
        n if n >= 600_000 => 214.0,
        n if n >= 400_000 => 75.0,
        n if n >= 11_000 => 27.0,
        n if n >= 890 => 10.0,
        n if n >= 177 => 3.33,
        n if n >= 24 => 1.33,
        _ => 0.33,
    }
}

/// Lookup epsilon for offset packing when north and south poles are reserved.
///
/// Separate table from [`packing_epsilon`], used when two samples are fixed at the poles
/// (Roberts pole lattice, equation 4 in the 2018 Extreme Learning articles).
pub fn packing_with_poles_epsilon(n: usize) -> f64 {
    match n {
        n if n >= 40_000 => 25.0,
        n if n >= 1_000 => 10.0,
        n if n >= 80 => 3.33,
        _ => 2.66,
    }
}

/// Fixed ε for Roberts' average-neighbor offset (≈ 0.36).
///
/// Improves max/min nearest-neighbor ratio versus canonical Fibonacci; see Baskerville
/// (2024) and Roberts (2018).
pub const AVERAGE_NEIGHBOR_EPSILON: f64 = 0.36;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_epsilon_low_tier() {
        assert_eq!(packing_epsilon(1), 0.33);
        assert_eq!(packing_epsilon(23), 0.33);
    }

    #[test]
    fn packing_epsilon_mid_tiers() {
        assert_eq!(packing_epsilon(24), 1.33);
        assert_eq!(packing_epsilon(176), 1.33);
        assert_eq!(packing_epsilon(177), 3.33);
        assert_eq!(packing_epsilon(889), 3.33);
        assert_eq!(packing_epsilon(890), 10.0);
        assert_eq!(packing_epsilon(10_999), 10.0);
        assert_eq!(packing_epsilon(11_000), 27.0);
        assert_eq!(packing_epsilon(399_999), 27.0);
        assert_eq!(packing_epsilon(400_000), 75.0);
        assert_eq!(packing_epsilon(599_999), 75.0);
        assert_eq!(packing_epsilon(600_000), 214.0);
    }

    #[test]
    fn poles_epsilon_all_tiers() {
        assert_eq!(packing_with_poles_epsilon(1), 2.66);
        assert_eq!(packing_with_poles_epsilon(79), 2.66);
        assert_eq!(packing_with_poles_epsilon(80), 3.33);
        assert_eq!(packing_with_poles_epsilon(999), 3.33);
        assert_eq!(packing_with_poles_epsilon(1_000), 10.0);
        assert_eq!(packing_with_poles_epsilon(39_999), 10.0);
        assert_eq!(packing_with_poles_epsilon(40_000), 25.0);
    }

    #[test]
    fn average_neighbor_epsilon_constant() {
        assert!((AVERAGE_NEIGHBOR_EPSILON - 0.36).abs() < f64::EPSILON);
    }
}
