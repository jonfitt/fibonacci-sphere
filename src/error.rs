//! Error types for lattice generation failures.

use thiserror::Error;

/// Errors returned by lattice generation.
#[derive(Debug, Error, PartialEq)]
pub enum SphereError {
    /// Point count must be at least 1.
    #[error("point count must be at least 1, got {n}")]
    InvalidPointCount {
        /// The invalid point count supplied.
        n: usize,
    },

    /// Radius must be strictly positive.
    #[error("radius must be positive, got {radius}")]
    InvalidRadius {
        /// The invalid radius supplied.
        radius: f64,
    },

    /// Vertex index is outside the lattice point range.
    #[error("vertex index {index} out of range (lattice has {count} points)")]
    InvalidVertexIndex {
        /// The invalid vertex index supplied.
        index: usize,
        /// Number of vertices in the lattice.
        count: usize,
    },

    /// No path exists between the two vertices on the Delaunay mesh graph.
    #[error("no surface path from vertex {from} to vertex {to}")]
    NoSurfacePath {
        /// Start vertex index.
        from: usize,
        /// End vertex index.
        to: usize,
    },

    /// Terrain-filtered routing requires a generated terrain map.
    #[error("terrain must be generated before terrain-filtered routing")]
    TerrainNotGenerated,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_point_count_display() {
        let err = SphereError::InvalidPointCount { n: 0 };
        assert_eq!(err.to_string(), "point count must be at least 1, got 0");
    }

    #[test]
    fn invalid_radius_display() {
        let err = SphereError::InvalidRadius { radius: -2.5 };
        assert_eq!(err.to_string(), "radius must be positive, got -2.5");
    }

    #[test]
    fn invalid_vertex_index_display() {
        let err = SphereError::InvalidVertexIndex { index: 5, count: 3 };
        assert_eq!(
            err.to_string(),
            "vertex index 5 out of range (lattice has 3 points)"
        );
    }

    #[test]
    fn no_surface_path_display() {
        let err = SphereError::NoSurfacePath { from: 0, to: 4 };
        assert_eq!(err.to_string(), "no surface path from vertex 0 to vertex 4");
    }

    #[test]
    fn terrain_not_generated_display() {
        let err = SphereError::TerrainNotGenerated;
        assert_eq!(
            err.to_string(),
            "terrain must be generated before terrain-filtered routing"
        );
    }
}
