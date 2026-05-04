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
    let vertices = points
        .iter()
        .enumerate()
        .map(|(index, point)| IndexedPoint {
            index,
            point: *point,
        })
        .collect::<Vec<_>>();

    let triangulation = DelaunayTriangulation::<IndexedPoint>::bulk_load_stable(vertices)
        .map_err(|error| AlphaShapeError::TriangulationFailed(error.to_string()))?;

    Ok(triangulation
        .inner_faces()
        .map(|face| {
            let vertices = face.vertices();
            Triangle {
                a: vertices[0].data().index,
                b: vertices[1].data().index,
                c: vertices[2].data().index,
            }
        })
        .collect())
}
