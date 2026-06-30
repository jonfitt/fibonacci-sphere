//! A sample point on the surface of a sphere.
//!
//! Coordinates use a **Y-up, right-handed** system compatible with Godot 4.

/// Golden ratio φ = (1 + √5) / 2.
pub const GOLDEN_RATIO: f64 = 1.618_033_988_749_895;

/// A single sample point on a sphere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpherePoint {
    /// Zero-based index in the generated sequence.
    pub index: usize,
    /// Cartesian position `[x, y, z]` scaled by the lattice radius. Y is up.
    pub position: [f32; 3],
    /// Azimuth angle in radians, `[0, 2π)`.
    pub theta: f64,
    /// Polar angle from the north pole (+Y) in radians, `[0, π]`.
    pub phi: f64,
}

impl SpherePoint {
    /// Build a point from spherical angles and radius (Y-up convention).
    pub(crate) fn from_spherical(index: usize, theta: f64, phi: f64, radius: f64) -> Self {
        let sin_phi = phi.sin();
        let cos_phi = phi.cos();
        let cos_theta = theta.cos();
        let sin_theta = theta.sin();

        let x = (radius * sin_phi * cos_theta) as f32;
        let y = (radius * cos_phi) as f32;
        let z = (radius * sin_phi * sin_theta) as f32;

        Self {
            index,
            position: [x, y, z],
            theta,
            phi,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::PI;

    use super::*;

    #[test]
    fn golden_ratio_value() {
        let expected = (1.0 + 5.0_f64.sqrt()) / 2.0;
        assert!((GOLDEN_RATIO - expected).abs() < 1e-12);
    }

    #[test]
    fn north_pole_is_y_up() {
        let p = SpherePoint::from_spherical(0, 0.0, 0.0, 1.0);
        assert!((p.position[1] - 1.0).abs() < 1e-5);
        assert!(p.position[0].abs() < 1e-5);
        assert!(p.position[2].abs() < 1e-5);
    }

    #[test]
    fn south_pole_is_negative_y() {
        let p = SpherePoint::from_spherical(0, 0.0, PI, 1.0);
        assert!((p.position[1] + 1.0).abs() < 1e-5);
        assert!(p.position[0].abs() < 1e-5);
        assert!(p.position[2].abs() < 1e-5);
    }

    #[test]
    fn equator_point_on_x_axis() {
        let p = SpherePoint::from_spherical(3, 0.0, PI / 2.0, 2.0);
        assert!((p.position[0] - 2.0).abs() < 1e-5);
        assert!(p.position[1].abs() < 1e-5);
        assert!(p.position[2].abs() < 1e-5);
    }

    #[test]
    fn radius_scales_position() {
        let unit = SpherePoint::from_spherical(1, 1.2, 0.8, 1.0);
        let scaled = SpherePoint::from_spherical(1, 1.2, 0.8, 5.0);
        assert!((scaled.position[0] - unit.position[0] * 5.0).abs() < 1e-4);
        assert!((scaled.position[1] - unit.position[1] * 5.0).abs() < 1e-4);
        assert!((scaled.position[2] - unit.position[2] * 5.0).abs() < 1e-4);
    }

    #[test]
    fn preserves_index_and_angles() {
        let p = SpherePoint::from_spherical(7, 1.1, 2.2, 1.0);
        assert_eq!(p.index, 7);
        assert!((p.theta - 1.1).abs() < 1e-12);
        assert!((p.phi - 2.2).abs() < 1e-12);
    }

    #[test]
    fn spherical_to_cartesian_roundtrip_on_surface() {
        let p = SpherePoint::from_spherical(0, 0.7, 1.1, 3.0);
        let [x, y, z] = p.position;
        let dist = ((x * x + y * y + z * z) as f64).sqrt();
        assert!((dist - 3.0).abs() < 1e-4);

        let recovered_phi = (y as f64 / 3.0).acos();
        assert!((recovered_phi - 1.1).abs() < 1e-4);
    }
}
