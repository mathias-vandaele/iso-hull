use std::time::Instant;

use log::trace;
use spade::{DelaunayTriangulation, HasPosition, Triangulation};

use crate::{error::AlphaShapeError, mesh::Triangle, types::Point2};

#[derive(Debug, Clone, Copy)]
struct IndexedPoint {
    index: usize,
    point: Point2,
}

impl HasPosition for IndexedPoint {
    type Scalar = f64;

    fn position(&self) -> spade::Point2<Self::Scalar> {
        spade::Point2::new(self.point.x, self.point.y)
    }
}

pub(crate) fn delaunay_triangles(points: &[Point2]) -> Result<Vec<Triangle>, AlphaShapeError> {
    let total_started = Instant::now();

    let started = Instant::now();
    let vertices = points
        .iter()
        .enumerate()
        .map(|(index, point)| IndexedPoint {
            index,
            point: *point,
        })
        .collect::<Vec<_>>();
    trace!(
        target: "isohull::triangulation",
        "build_vertices points={} vertices={} elapsed_ms={:.3}",
        points.len(),
        vertices.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangulation = DelaunayTriangulation::<IndexedPoint>::bulk_load_stable(vertices)
        .map_err(|error| AlphaShapeError::TriangulationFailed(error.to_string()))?;
    trace!(
        target: "isohull::triangulation",
        "bulk_load_stable elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangles = triangulation
        .inner_faces()
        .map(|face| {
            let vertices = face.vertices();
            Triangle {
                a: vertices[0].data().index,
                b: vertices[1].data().index,
                c: vertices[2].data().index,
            }
        })
        .collect::<Vec<_>>();
    trace!(
        target: "isohull::triangulation",
        "collect_inner_faces triangles={} elapsed_ms={:.3}",
        triangles.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    trace!(
        target: "isohull::triangulation",
        "delaunay_triangles.total points={} triangles={} elapsed_ms={:.3}",
        points.len(),
        triangles.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(triangles)
}
