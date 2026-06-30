use crate::error::SphereError;

/// Validates point count and radius before lattice generation.
pub(crate) fn validate_lattice_params(n: usize, radius: f64) -> Result<(), SphereError> {
    if n == 0 {
        return Err(SphereError::InvalidPointCount { n });
    }
    if radius <= 0.0 {
        return Err(SphereError::InvalidRadius { radius });
    }
    Ok(())
}
