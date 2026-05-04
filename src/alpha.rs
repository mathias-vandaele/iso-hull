use crate::{
    error::AlphaShapeError,
    geometry::{circumradius, percentile_sorted, total_area},
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

    let reference_alpha = percentile_sorted(&radii, 0.995).unwrap_or(radii[radii.len() - 1]);
    let reference_shape = build_alpha_shape(points, triangles, reference_alpha)
        .or_else(|_| build_alpha_shape(points, triangles, radii[radii.len() - 1]))?;
    let reference_area = total_area(&reference_shape);
    let target_polygon_count = reference_shape.polygons.len().max(1);

    let candidate_percentiles = [
        0.50, 0.60, 0.70, 0.80, 0.85, 0.90, 0.93, 0.95, 0.97, 0.98, 0.99, 0.995,
    ];

    let mut last_alpha = None;
    for percentile in candidate_percentiles {
        let Some(alpha) = percentile_sorted(&radii, percentile) else {
            continue;
        };

        if last_alpha == Some(alpha) {
            continue;
        }
        last_alpha = Some(alpha);

        let Ok(shape) = build_alpha_shape(points, triangles, alpha) else {
            continue;
        };

        if shape.polygons.len() <= target_polygon_count
            && total_area(&shape) >= reference_area * 0.85
        {
            return Ok(alpha);
        }
    }

    Ok(reference_alpha)
}

pub(crate) fn triangle_circumradius(triangle: Triangle, points: &[Point2]) -> Option<f64> {
    circumradius(points[triangle.a], points[triangle.b], points[triangle.c])
}
