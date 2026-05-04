use crate::{
    error::AlphaShapeError,
    geometry::{circumradius, percentile_sorted},
    mesh::Triangle,
    polygonize::polygons_from_triangles,
    preprocess::prepare_points,
    triangulation::delaunay_triangles,
    types::{MultiPolygon, Point2},
};

pub fn alpha_shape(
    points: impl Into<Vec<Point2>>,
    alpha_radius: f64,
) -> Result<MultiPolygon, AlphaShapeError> {
    if !alpha_radius.is_finite() || alpha_radius <= 0.0 {
        return Err(AlphaShapeError::InvalidAlpha);
    }

    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;
    build_alpha_shape(&points, &triangles, alpha_radius)
}

pub fn alpha_shape_auto(points: impl Into<Vec<Point2>>) -> Result<MultiPolygon, AlphaShapeError> {
    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;
    let alpha_radius = estimate_alpha_radius_from_triangles(&points, &triangles)?;

    build_alpha_shape(&points, &triangles, alpha_radius)
}

pub fn estimate_alpha_radius(points: impl Into<Vec<Point2>>) -> Result<f64, AlphaShapeError> {
    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;

    estimate_alpha_radius_from_triangles(&points, &triangles)
}

pub(crate) fn build_alpha_shape(
    points: &[Point2],
    triangles: &[Triangle],
    alpha_radius: f64,
) -> Result<MultiPolygon, AlphaShapeError> {
    let kept_triangles = triangles
        .iter()
        .copied()
        .filter(|triangle| {
            triangle_circumradius(*triangle, points).is_some_and(|radius| radius <= alpha_radius)
        })
        .collect::<Vec<_>>();

    if kept_triangles.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    let polygons = polygons_from_triangles(&kept_triangles, points);
    if polygons.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    Ok(MultiPolygon { polygons })
}

pub(crate) fn estimate_alpha_radius_from_triangles(
    points: &[Point2],
    triangles: &[Triangle],
) -> Result<f64, AlphaShapeError> {
    let mut radii = triangles
        .iter()
        .filter_map(|triangle| triangle_circumradius(*triangle, points))
        .filter(|radius| radius.is_finite() && *radius > 0.0)
        .collect::<Vec<_>>();

    if radii.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    radii.sort_by(f64::total_cmp);

    Ok(estimate_alpha_radius_from_sorted_radii(&radii))
}

pub(crate) fn triangle_circumradius(triangle: Triangle, points: &[Point2]) -> Option<f64> {
    circumradius(points[triangle.a], points[triangle.b], points[triangle.c])
}

fn estimate_alpha_radius_from_sorted_radii(radii: &[f64]) -> f64 {
    percentile_sorted(radii, 0.995).unwrap_or(radii[radii.len() - 1])
}
