//! Surface topology for sphere lattices (spherical Delaunay wireframe).

mod delaunay2d;
mod pathfinding;
mod spherical;
pub mod voronoi;
pub mod voronoi_mesh;

pub use pathfinding::{geodesic_distance, nearest_vertex_index, SurfaceGraph, SurfacePath};
pub use spherical::{
    spherical_delaunay_edges, spherical_delaunay_mesh, spherical_delaunay_triangles,
    SphericalMesh,
};
pub use voronoi::{
    spherical_voronoi_border_segments, spherical_voronoi_cells, voronoi_cell_fan_apex,
    VoronoiCell,
};
pub use voronoi_mesh::{
    build_voronoi_cell_fan_mesh, coincident_on_sphere, is_geographic_pole_apex,
    mesh_boundary_ring, VoronoiFanMesh, VoronoiFanMeshOptions, DEFAULT_SURFACE_INSET,
};

/// Undirected edge index pairs for [`spherical_delaunay_edges`].
pub type WireframeEdge = [usize; 2];

/// Build line-segment endpoints for wireframe rendering: `[a, b, c, d, ...]`.
pub fn wireframe_segment_positions(
    positions: &[[f32; 3]],
    edges: &[[usize; 2]],
) -> Vec<[f32; 3]> {
    let mut segments = Vec::with_capacity(edges.len() * 2);
    for edge in edges {
        segments.push(positions[edge[0]]);
        segments.push(positions[edge[1]]);
    }
    segments
}
