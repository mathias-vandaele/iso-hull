use std::time::Instant;

use log::trace;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    error::AlphaShapeError,
    types::{GeoMultiPolygon, GeoPolygon, LatLon, MultiPolygon, Point2},
};

const EARTH_RADIUS_METERS: f64 = 6_371_000.0;

pub(crate) fn prepare_points(points: Vec<Point2>) -> Result<Vec<Point2>, AlphaShapeError> {
    let started = Instant::now();
    let input_points = points.len();
    let mut unique_points = Vec::new();
    let mut seen = FxHashSet::default();
    seen.reserve(input_points);

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

pub(crate) fn validate_points(points: &[Point2]) -> Result<(), AlphaShapeError> {
    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(AlphaShapeError::InvalidPoint);
        }
    }

    Ok(())
}

pub(crate) fn spatial_subsample(points: &[Point2], max_points: usize) -> Vec<Point2> {
    let started = Instant::now();
    let input_points = points.len();
    let grid_dim = grid_dimension(max_points);

    if input_points == 0 || max_points == 0 {
        trace_spatial_subsample(input_points, max_points, grid_dim, 0, started);
        return Vec::new();
    }

    if input_points <= max_points {
        trace_spatial_subsample(input_points, max_points, grid_dim, input_points, started);
        return points.to_vec();
    }

    let Some(bounds) = Bounds::from_points(points) else {
        trace_spatial_subsample(input_points, max_points, grid_dim, 0, started);
        return Vec::new();
    };

    let projection = GridProjection::new(bounds, grid_dim);

    let mut grid = FxHashMap::with_capacity_and_hasher(max_points, Default::default());
    let mut selected = Vec::with_capacity(max_points);

    push_extreme(
        bounds.min_x_index,
        points,
        projection,
        &mut grid,
        &mut selected,
    );
    push_extreme(
        bounds.max_x_index,
        points,
        projection,
        &mut grid,
        &mut selected,
    );
    push_extreme(
        bounds.min_y_index,
        points,
        projection,
        &mut grid,
        &mut selected,
    );
    push_extreme(
        bounds.max_y_index,
        points,
        projection,
        &mut grid,
        &mut selected,
    );

    if selected.len() < max_points {
        for (index, point) in points.iter().enumerate() {
            let key = projection.point_key(*point);

            if grid.contains_key(&key) {
                continue;
            }

            grid.insert(key, index);
            selected.push(index);

            if selected.len() == max_points {
                break;
            }
        }
    }

    let mut output = Vec::with_capacity(selected.len());
    for index in selected {
        output.push(points[index]);
    }

    trace_spatial_subsample(input_points, max_points, grid_dim, output.len(), started);

    output
}

pub(crate) fn project_lat_lon(points: &[LatLon]) -> ProjectedLatLon {
    let started = Instant::now();

    let Some(projection) = LatLonProjection::from_points(points) else {
        return ProjectedLatLon {
            points: Vec::new(),
            projection: None,
        };
    };

    let projected = points
        .iter()
        .map(|point| projection.project(*point))
        .collect::<Vec<_>>();

    trace!(
        target: "isohull::preprocess",
        "project_lat_lon input_points={} projected_points={} elapsed_ms={:.3}",
        points.len(),
        projected.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    ProjectedLatLon {
        points: projected,
        projection: Some(projection),
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ProjectedLatLon {
    pub(crate) points: Vec<Point2>,
    pub(crate) projection: Option<LatLonProjection>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LatLonProjection {
    origin: LatLon,
    longitude_scale: f64,
}

impl LatLonProjection {
    fn from_points(points: &[LatLon]) -> Option<Self> {
        let origin = points.first().copied()?;
        let mean_latitude = points.iter().map(|point| point.lat).sum::<f64>() / points.len() as f64;

        Some(Self {
            origin,
            longitude_scale: mean_latitude.to_radians().cos(),
        })
    }

    fn project(self, point: LatLon) -> Point2 {
        let x =
            (point.lon - self.origin.lon).to_radians() * EARTH_RADIUS_METERS * self.longitude_scale;
        let y = (point.lat - self.origin.lat).to_radians() * EARTH_RADIUS_METERS;

        Point2::new(x, y)
    }

    pub(crate) fn unproject_multi_polygon(self, shape: MultiPolygon) -> GeoMultiPolygon {
        let polygons = shape
            .polygons
            .into_iter()
            .map(|polygon| GeoPolygon {
                outer: polygon
                    .outer
                    .into_iter()
                    .map(|point| self.unproject(point))
                    .collect(),
            })
            .collect();

        GeoMultiPolygon { polygons }
    }

    fn unproject(self, point: Point2) -> LatLon {
        let lat = self.origin.lat + (point.y / EARTH_RADIUS_METERS).to_degrees();
        let lon = if self.longitude_scale.abs() > f64::EPSILON {
            self.origin.lon + (point.x / (EARTH_RADIUS_METERS * self.longitude_scale)).to_degrees()
        } else {
            self.origin.lon
        };

        LatLon::new(lat, lon)
    }
}

fn push_extreme(
    index: usize,
    points: &[Point2],
    projection: GridProjection,
    grid: &mut FxHashMap<i64, usize>,
    selected: &mut Vec<usize>,
) {
    if selected.len() == selected.capacity() || selected.contains(&index) {
        return;
    }

    let key = projection.point_key(points[index]);
    grid.entry(key).or_insert(index);
    selected.push(index);
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_x_index: usize,
    max_x_index: usize,
    min_y_index: usize,
    max_y_index: usize,
}

#[derive(Debug, Clone, Copy)]
struct GridProjection {
    bounds: Bounds,
    grid_dim: usize,
    x_scale: f64,
    y_scale: f64,
}

impl GridProjection {
    fn new(bounds: Bounds, grid_dim: usize) -> Self {
        Self {
            bounds,
            grid_dim,
            x_scale: axis_scale(bounds.min_x, bounds.max_x, grid_dim),
            y_scale: axis_scale(bounds.min_y, bounds.max_y, grid_dim),
        }
    }

    #[inline(always)]
    fn point_key(self, point: Point2) -> i64 {
        let ix = cell_index(point.x, self.bounds.min_x, self.x_scale, self.grid_dim);
        let iy = cell_index(point.y, self.bounds.min_y, self.y_scale, self.grid_dim);
        cell_key(ix, iy)
    }
}

impl Bounds {
    fn from_points(points: &[Point2]) -> Option<Self> {
        let first = points.first().copied()?;
        let mut bounds = Self {
            min_x: first.x,
            max_x: first.x,
            min_y: first.y,
            max_y: first.y,
            min_x_index: 0,
            max_x_index: 0,
            min_y_index: 0,
            max_y_index: 0,
        };

        for (index, point) in points.iter().copied().enumerate().skip(1) {
            if point.x < bounds.min_x {
                bounds.min_x = point.x;
                bounds.min_x_index = index;
            }
            if point.x > bounds.max_x {
                bounds.max_x = point.x;
                bounds.max_x_index = index;
            }
            if point.y < bounds.min_y {
                bounds.min_y = point.y;
                bounds.min_y_index = index;
            }
            if point.y > bounds.max_y {
                bounds.max_y = point.y;
                bounds.max_y_index = index;
            }
        }

        Some(bounds)
    }
}

fn trace_spatial_subsample(
    input_points: usize,
    max_points: usize,
    grid_dim: usize,
    output_points: usize,
    started: Instant,
) {
    trace!(
        target: "isohull::preprocess",
        "spatial_subsample input_points={} max_points={} grid_dim={} output_points={} elapsed_ms={:.3}",
        input_points,
        max_points,
        grid_dim,
        output_points,
        started.elapsed().as_secs_f64() * 1000.0
    );
}

#[inline(always)]
fn grid_dimension(max_points: usize) -> usize {
    (max_points as f64).sqrt().floor().max(1.0) as usize
}

#[inline(always)]
fn axis_scale(min: f64, max: f64, grid_dim: usize) -> f64 {
    let width = max - min;
    if width > 0.0 {
        grid_dim as f64 / width
    } else {
        0.0
    }
}

#[inline(always)]
fn cell_index(value: f64, min: f64, scale: f64, grid_dim: usize) -> i32 {
    if scale == 0.0 {
        0
    } else {
        let index = ((value - min) * scale).floor() as i32;
        index.clamp(0, grid_dim as i32 - 1)
    }
}

#[inline(always)]
fn cell_key(ix: i32, iy: i32) -> i64 {
    ((ix as i64) << 32) | (iy as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_subsample_respects_max_points() {
        let points = grid_points(100, 100);
        let output = spatial_subsample(&points, 1_000);

        assert!(output.len() <= 1_000);
    }

    #[test]
    fn spatial_subsample_preserves_bounding_box_when_possible() {
        let points = grid_points(20, 20);
        let output = spatial_subsample(&points, 100);

        assert_eq!(bounds_tuple(&output), bounds_tuple(&points));
    }

    #[test]
    fn spatial_subsample_handles_degenerate_inputs() {
        assert!(spatial_subsample(&[], 10).is_empty());

        let one = [Point2::new(1.0, -1.0)];
        assert_eq!(spatial_subsample(&one, 10), one);

        let two = [Point2::new(1.0, -1.0), Point2::new(2.0, -2.0)];
        assert_eq!(spatial_subsample(&two, 10), two);

        let three = [
            Point2::new(1.0, -1.0),
            Point2::new(2.0, -2.0),
            Point2::new(3.0, -3.0),
        ];
        assert_eq!(spatial_subsample(&three, 10), three);

        let identical = vec![Point2::new(5.0, 5.0); 100];
        let output = spatial_subsample(&identical, 10);
        assert_eq!(output, vec![Point2::new(5.0, 5.0)]);
    }

    #[test]
    fn spatial_subsample_is_deterministic() {
        let points = grid_points(70, 55);

        assert_eq!(
            spatial_subsample(&points, 500),
            spatial_subsample(&points, 500)
        );
    }

    fn grid_points(width: usize, height: usize) -> Vec<Point2> {
        let mut points = Vec::with_capacity(width * height);
        for y in 0..height {
            for x in 0..width {
                points.push(Point2::new(x as f64 - 50.0, y as f64 * 1.5 - 20.0));
            }
        }

        points
    }

    fn bounds_tuple(points: &[Point2]) -> (u64, u64, u64, u64) {
        let bounds = Bounds::from_points(points).expect("points should not be empty");
        (
            bounds.min_x.to_bits(),
            bounds.max_x.to_bits(),
            bounds.min_y.to_bits(),
            bounds.max_y.to_bits(),
        )
    }
}
