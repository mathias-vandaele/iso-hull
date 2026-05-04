use std::time::Instant;

use log::trace;

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
    let total_started = Instant::now();

    if !alpha_radius.is_finite() || alpha_radius <= 0.0 {
        return Err(AlphaShapeError::InvalidAlpha);
    }

    let started = Instant::now();
    let points = prepare_points(points.into())?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape.prepare_points points={} elapsed_ms={:.3}",
        points.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangles = delaunay_triangles(&points)?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape.delaunay_triangles triangles={} elapsed_ms={:.3}",
        triangles.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let shape = build_alpha_shape(&points, &triangles, alpha_radius)?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape.total polygons={} elapsed_ms={:.3}",
        shape.polygons.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(shape)
}

pub fn alpha_shape_auto(points: impl Into<Vec<Point2>>) -> Result<MultiPolygon, AlphaShapeError> {
    let total_started = Instant::now();

    let started = Instant::now();
    let points = prepare_points(points.into())?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape_auto.prepare_points points={} elapsed_ms={:.3}",
        points.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangles = delaunay_triangles(&points)?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape_auto.delaunay_triangles triangles={} elapsed_ms={:.3}",
        triangles.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let alpha_radius = estimate_alpha_radius_from_triangles(&points, &triangles)?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape_auto.estimate_alpha_radius alpha_radius={:.6} elapsed_ms={:.3}",
        alpha_radius,
        started.elapsed().as_secs_f64() * 1000.0
    );

    let shape = build_alpha_shape(&points, &triangles, alpha_radius)?;
    trace!(
        target: "isohull::alpha",
        "alpha_shape_auto.total polygons={} elapsed_ms={:.3}",
        shape.polygons.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(shape)
}

pub fn estimate_alpha_radius(points: impl Into<Vec<Point2>>) -> Result<f64, AlphaShapeError> {
    let total_started = Instant::now();

    let started = Instant::now();
    let points = prepare_points(points.into())?;
    trace!(
        target: "isohull::alpha",
        "estimate_alpha_radius.prepare_points points={} elapsed_ms={:.3}",
        points.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangles = delaunay_triangles(&points)?;
    trace!(
        target: "isohull::alpha",
        "estimate_alpha_radius.delaunay_triangles triangles={} elapsed_ms={:.3}",
        triangles.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let alpha_radius = estimate_alpha_radius_from_triangles(&points, &triangles)?;
    trace!(
        target: "isohull::alpha",
        "estimate_alpha_radius.total alpha_radius={:.6} elapsed_ms={:.3}",
        alpha_radius,
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(alpha_radius)
}

pub(crate) fn build_alpha_shape(
    points: &[Point2],
    triangles: &[Triangle],
    alpha_radius: f64,
) -> Result<MultiPolygon, AlphaShapeError> {
    let total_started = Instant::now();

    let started = Instant::now();
    let kept_triangles = triangles
        .iter()
        .copied()
        .filter(|triangle| {
            triangle_circumradius(*triangle, points).is_some_and(|radius| radius <= alpha_radius)
        })
        .collect::<Vec<_>>();
    trace!(
        target: "isohull::alpha",
        "filter_triangles input_triangles={} kept_triangles={} alpha_radius={:.6} elapsed_ms={:.3}",
        triangles.len(),
        kept_triangles.len(),
        alpha_radius,
        started.elapsed().as_secs_f64() * 1000.0
    );

    if kept_triangles.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    let started = Instant::now();
    let polygons = polygons_from_triangles(&kept_triangles, points);
    trace!(
        target: "isohull::alpha",
        "polygonize polygons={} elapsed_ms={:.3}",
        polygons.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );
    if polygons.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    trace!(
        target: "isohull::alpha",
        "build_alpha_shape.total polygons={} elapsed_ms={:.3}",
        polygons.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(MultiPolygon { polygons })
}

pub(crate) fn estimate_alpha_radius_from_triangles(
    points: &[Point2],
    triangles: &[Triangle],
) -> Result<f64, AlphaShapeError> {
    let total_started = Instant::now();

    let started = Instant::now();
    let mut radii = triangles
        .iter()
        .filter_map(|triangle| triangle_circumradius(*triangle, points))
        .filter(|radius| radius.is_finite() && *radius > 0.0)
        .collect::<Vec<_>>();
    trace!(
        target: "isohull::alpha",
        "estimate_alpha.collect_radii triangles={} radii={} elapsed_ms={:.3}",
        triangles.len(),
        radii.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    if radii.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    let started = Instant::now();
    radii.sort_by(f64::total_cmp);
    trace!(
        target: "isohull::alpha",
        "estimate_alpha.sort_radii radii={} elapsed_ms={:.3}",
        radii.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let alpha_radius = estimate_alpha_radius_from_sorted_radii(&radii);
    trace!(
        target: "isohull::alpha",
        "estimate_alpha.total alpha_radius={:.6} elapsed_ms={:.3}",
        alpha_radius,
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(alpha_radius)
}

pub(crate) fn triangle_circumradius(triangle: Triangle, points: &[Point2]) -> Option<f64> {
    circumradius(points[triangle.a], points[triangle.b], points[triangle.c])
}

fn estimate_alpha_radius_from_sorted_radii(radii: &[f64]) -> f64 {
    percentile_sorted(radii, 0.995).unwrap_or(radii[radii.len() - 1])
}
