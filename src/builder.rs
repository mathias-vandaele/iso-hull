use crate::{
    alpha::{build_alpha_shape, estimate_alpha_radius_from_triangles},
    error::IsoHullError,
    geometry::signed_area,
    preprocess::{prepare_points, project_lat_lon},
    triangulation::delaunay_triangles,
    types::{LatLon, MultiPolygon, Point2, Polygon},
};

pub struct IsoHull;

pub struct IsoHullInputBuilder {
    points: Vec<Point2>,
}

pub struct IsoHullAreaBuilder {
    points: Vec<Point2>,
    alpha: AlphaSelection,
}

pub struct IsoHullBuildBuilder {
    points: Vec<Point2>,
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
        IsoHullInputBuilder {
            points: points.into_iter().map(Into::into).collect(),
        }
    }

    pub fn from_lat_lon<I, P>(points: I) -> IsoHullInputBuilder
    where
        I: IntoIterator<Item = P>,
        P: Into<LatLon>,
    {
        let points = points.into_iter().map(Into::into).collect::<Vec<_>>();
        IsoHullInputBuilder {
            points: project_lat_lon(&points),
        }
    }
}

impl IsoHullInputBuilder {
    pub fn auto_alpha(self) -> IsoHullAreaBuilder {
        IsoHullAreaBuilder {
            points: self.points,
            alpha: AlphaSelection::Auto,
        }
    }

    pub fn alpha(self, alpha_radius: f64) -> IsoHullAreaBuilder {
        IsoHullAreaBuilder {
            points: self.points,
            alpha: AlphaSelection::Manual(alpha_radius),
        }
    }
}

impl IsoHullAreaBuilder {
    pub fn min_area_ratio(self, ratio: f64) -> IsoHullBuildBuilder {
        IsoHullBuildBuilder {
            points: self.points,
            alpha: self.alpha,
            area_filter: AreaFilter::MinRatio(ratio),
        }
    }

    pub fn all_area(self) -> IsoHullBuildBuilder {
        IsoHullBuildBuilder {
            points: self.points,
            alpha: self.alpha,
            area_filter: AreaFilter::All,
        }
    }
}

impl IsoHullBuildBuilder {
    pub fn build(self) -> Result<MultiPolygon, IsoHullError> {
        validate_area_filter(self.area_filter)?;

        let points = prepare_points(self.points)?;
        let triangles = delaunay_triangles(&points)?;
        let alpha_radius = match self.alpha {
            AlphaSelection::Auto => estimate_alpha_radius_from_triangles(&points, &triangles)?,
            AlphaSelection::Manual(alpha_radius) => {
                if !alpha_radius.is_finite() || alpha_radius <= 0.0 {
                    return Err(IsoHullError::InvalidAlpha);
                }

                alpha_radius
            }
        };

        let mut shape = build_alpha_shape(&points, &triangles, alpha_radius)?;
        apply_area_filter(&mut shape.polygons, self.area_filter);

        if shape.polygons.is_empty() {
            return Err(IsoHullError::EmptyShape);
        }

        Ok(shape)
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
