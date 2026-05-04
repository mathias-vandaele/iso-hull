use std::{collections::HashSet, time::Instant};

use log::trace;

use crate::{
    error::AlphaShapeError,
    types::{LatLon, Point2},
};

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

pub(crate) fn prepare_points(points: Vec<Point2>) -> Result<Vec<Point2>, AlphaShapeError> {
    let started = Instant::now();
    let input_points = points.len();
    let mut unique_points = Vec::new();
    let mut seen = HashSet::new();

    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(AlphaShapeError::InvalidPoint);
        }

        if seen.insert((point.x.to_bits(), point.y.to_bits())) {
            unique_points.push(point);
        }
    }

    if unique_points.len() < 3 {
        return Err(AlphaShapeError::NotEnoughPoints(unique_points.len()));
    }

    trace!(
        target: "isohull::preprocess",
        "prepare_points input_points={} unique_points={} elapsed_ms={:.3}",
        input_points,
        unique_points.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(unique_points)
}

pub(crate) fn project_lat_lon(points: &[LatLon]) -> Vec<Point2> {
    let started = Instant::now();

    let Some(origin) = points.first().copied() else {
        return Vec::new();
    };

    let mean_latitude = points.iter().map(|point| point.lat).sum::<f64>() / points.len() as f64;
    let longitude_scale = mean_latitude.to_radians().cos();

    let projected = points
        .iter()
        .map(|point| {
            let x = (point.lon - origin.lon).to_radians() * EARTH_RADIUS_METERS * longitude_scale;
            let y = (point.lat - origin.lat).to_radians() * EARTH_RADIUS_METERS;
            Point2::new(x, y)
        })
        .collect::<Vec<_>>();

    trace!(
        target: "isohull::preprocess",
        "project_lat_lon input_points={} projected_points={} elapsed_ms={:.3}",
        points.len(),
        projected.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    projected
}
