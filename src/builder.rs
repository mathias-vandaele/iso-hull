use std::time::Instant;

use log::trace;

use crate::{
    alpha::{build_alpha_shape, estimate_alpha_radius_from_triangles},
    error::IsoHullError,
    geometry::signed_area,
    preprocess::{
        prepare_points, project_lat_lon, spatial_subsample, validate_points, LatLonProjection,
    },
    triangulation::delaunay_triangles,
    types::{GeoMultiPolygon, LatLon, MultiPolygon, Point2, Polygon},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HullMode {
    Low,
    Medium,
    High,
    Ultra,
    #[default]
    Exact,
}

pub struct IsoHull;

pub struct IsoHullInputBuilder {
    points: Vec<Point2>,
    mode: HullMode,
}

pub struct IsoHullLatLonInputBuilder {
    points: Vec<Point2>,
    projection: Option<LatLonProjection>,
    mode: HullMode,
}

pub struct IsoHullAreaBuilder {
    points: Vec<Point2>,
    mode: HullMode,
    alpha: AlphaSelection,
}

pub struct IsoHullLatLonAreaBuilder {
    points: Vec<Point2>,
    projection: Option<LatLonProjection>,
    mode: HullMode,
    alpha: AlphaSelection,
}

pub struct IsoHullBuildBuilder {
    points: Vec<Point2>,
    mode: HullMode,
    alpha: AlphaSelection,
    area_filter: AreaFilter,
}

pub struct IsoHullLatLonBuildBuilder {
    points: Vec<Point2>,
    projection: Option<LatLonProjection>,
    mode: HullMode,
    alpha: AlphaSelection,
    area_filter: AreaFilter,
}

#[derive(Debug, Clone, Copy)]
enum AlphaSelection {
    Auto,
    Manual(f64),
}

#[derive(Debug, Clone, Copy)]
enum AreaFilter {
    All,
    MinRatio(f64),
}

impl IsoHull {
    pub fn from_xy<I, P>(points: I) -> IsoHullInputBuilder
    where
        I: IntoIterator<Item = P>,
        P: Into<Point2>,
    {
        let started = Instant::now();
        IsoHullInputBuilder {
            points: points.into_iter().map(Into::into).collect(),
            mode: HullMode::Exact,
        }
        .tap_trace_from_xy(started)
    }

    pub fn from_lat_lon<I, P>(points: I) -> IsoHullLatLonInputBuilder
    where
        I: IntoIterator<Item = P>,
        P: Into<LatLon>,
    {
        let started = Instant::now();
        let points = points.into_iter().map(Into::into).collect::<Vec<_>>();
        let projected = project_lat_lon(&points);
        IsoHullLatLonInputBuilder {
            points: projected.points,
            projection: projected.projection,
            mode: HullMode::Exact,
        }
        .tap_trace_from_lat_lon(started, points.len())
    }
}

impl IsoHullInputBuilder {
    fn tap_trace_from_xy(self, started: Instant) -> Self {
        trace!(
            target: "isohull::builder",
            "from_xy input_points={} elapsed_ms={:.3}",
            self.points.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        self
    }
}

impl IsoHullLatLonInputBuilder {
    fn tap_trace_from_lat_lon(self, started: Instant, input_points: usize) -> Self {
        trace!(
            target: "isohull::builder",
            "from_lat_lon input_points={} projected_points={} elapsed_ms={:.3}",
            input_points,
            self.points.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
        self
    }
}

impl IsoHullInputBuilder {
    pub fn mode(self, mode: HullMode) -> Self {
        Self { mode, ..self }
    }

    pub fn auto_alpha(self) -> IsoHullAreaBuilder {
        IsoHullAreaBuilder {
            points: self.points,
            mode: self.mode,
            alpha: AlphaSelection::Auto,
        }
    }

    pub fn alpha(self, alpha_radius: f64) -> IsoHullAreaBuilder {
        IsoHullAreaBuilder {
            points: self.points,
            mode: self.mode,
            alpha: AlphaSelection::Manual(alpha_radius),
        }
    }
}

impl IsoHullLatLonInputBuilder {
    pub fn mode(self, mode: HullMode) -> Self {
        Self { mode, ..self }
    }

    pub fn auto_alpha(self) -> IsoHullLatLonAreaBuilder {
        IsoHullLatLonAreaBuilder {
            points: self.points,
            projection: self.projection,
            mode: self.mode,
            alpha: AlphaSelection::Auto,
        }
    }

    pub fn alpha(self, alpha_radius: f64) -> IsoHullLatLonAreaBuilder {
        IsoHullLatLonAreaBuilder {
            points: self.points,
            projection: self.projection,
            mode: self.mode,
            alpha: AlphaSelection::Manual(alpha_radius),
        }
    }
}

impl IsoHullAreaBuilder {
    pub fn min_area_ratio(self, ratio: f64) -> IsoHullBuildBuilder {
        IsoHullBuildBuilder {
            points: self.points,
            mode: self.mode,
            alpha: self.alpha,
            area_filter: AreaFilter::MinRatio(ratio),
        }
    }

    pub fn all_area(self) -> IsoHullBuildBuilder {
        IsoHullBuildBuilder {
            points: self.points,
            mode: self.mode,
            alpha: self.alpha,
            area_filter: AreaFilter::All,
        }
    }
}

impl IsoHullLatLonAreaBuilder {
    pub fn min_area_ratio(self, ratio: f64) -> IsoHullLatLonBuildBuilder {
        IsoHullLatLonBuildBuilder {
            points: self.points,
            projection: self.projection,
            mode: self.mode,
            alpha: self.alpha,
            area_filter: AreaFilter::MinRatio(ratio),
        }
    }

    pub fn all_area(self) -> IsoHullLatLonBuildBuilder {
        IsoHullLatLonBuildBuilder {
            points: self.points,
            projection: self.projection,
            mode: self.mode,
            alpha: self.alpha,
            area_filter: AreaFilter::All,
        }
    }
}

impl IsoHullBuildBuilder {
    pub fn build(self) -> Result<MultiPolygon, IsoHullError> {
        build_projected_multi_polygon(self.points, self.mode, self.alpha, self.area_filter)
    }
}

impl IsoHullLatLonBuildBuilder {
    pub fn build(self) -> Result<GeoMultiPolygon, IsoHullError> {
        let shape =
            build_projected_multi_polygon(self.points, self.mode, self.alpha, self.area_filter)?;
        let projection = self
            .projection
            .expect("projection should exist when a lat/lon hull can be built");

        Ok(projection.unproject_multi_polygon(shape))
    }
}

fn build_projected_multi_polygon(
    points: Vec<Point2>,
    mode: HullMode,
    alpha: AlphaSelection,
    area_filter: AreaFilter,
) -> Result<MultiPolygon, IsoHullError> {
    let total_started = Instant::now();

    let started = Instant::now();
    validate_area_filter(area_filter)?;
    trace!(
        target: "isohull::builder",
        "validate_area_filter elapsed_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let points = prepare_build_points(points, mode)?;
    trace!(
        target: "isohull::builder",
        "prepare_points points={} elapsed_ms={:.3}",
        points.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let triangles = delaunay_triangles(&points)?;
    trace!(
        target: "isohull::builder",
        "delaunay_triangles triangles={} elapsed_ms={:.3}",
        triangles.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let alpha_radius = match alpha {
        AlphaSelection::Auto => estimate_alpha_radius_from_triangles(&points, &triangles)?,
        AlphaSelection::Manual(alpha_radius) => {
            if !alpha_radius.is_finite() || alpha_radius <= 0.0 {
                return Err(IsoHullError::InvalidAlpha);
            }

            alpha_radius
        }
    };
    trace!(
        target: "isohull::builder",
        "resolve_alpha alpha_radius={:.6} elapsed_ms={:.3}",
        alpha_radius,
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let mut shape = build_alpha_shape(&points, &triangles, alpha_radius)?;
    trace!(
        target: "isohull::builder",
        "build_alpha_shape polygons={} elapsed_ms={:.3}",
        shape.polygons.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let polygons_before_filter = shape.polygons.len();
    apply_area_filter(&mut shape.polygons, area_filter);
    trace!(
        target: "isohull::builder",
        "apply_area_filter polygons_before={} polygons_after={} elapsed_ms={:.3}",
        polygons_before_filter,
        shape.polygons.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    if shape.polygons.is_empty() {
        return Err(IsoHullError::EmptyShape);
    }

    trace!(
        target: "isohull::builder",
        "build_total polygons={} elapsed_ms={:.3}",
        shape.polygons.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    Ok(shape)
}

fn prepare_build_points(points: Vec<Point2>, mode: HullMode) -> Result<Vec<Point2>, IsoHullError> {
    if let Some(max_points) = mode.max_points() {
        validate_points(&points)?;
        prepare_points(spatial_subsample(&points, max_points))
    } else {
        prepare_points(points)
    }
}

impl HullMode {
    fn max_points(self) -> Option<usize> {
        match self {
            Self::Low => Some(10_000),
            Self::Medium => Some(50_000),
            Self::High => Some(100_000),
            Self::Ultra => Some(250_000),
            Self::Exact => None,
        }
    }
}

fn validate_area_filter(area_filter: AreaFilter) -> Result<(), IsoHullError> {
    match area_filter {
        AreaFilter::All => Ok(()),
        AreaFilter::MinRatio(ratio) if ratio.is_finite() && (0.0..=1.0).contains(&ratio) => Ok(()),
        AreaFilter::MinRatio(_) => Err(IsoHullError::InvalidAreaRatio),
    }
}

fn apply_area_filter(polygons: &mut Vec<Polygon>, area_filter: AreaFilter) {
    let AreaFilter::MinRatio(ratio) = area_filter else {
        return;
    };

    let largest_area = polygons
        .iter()
        .map(|polygon| signed_area(&polygon.outer).abs())
        .fold(0.0, f64::max);
    let min_area = largest_area * ratio;

    polygons.retain(|polygon| signed_area(&polygon.outer).abs() >= min_area);
}
