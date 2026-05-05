mod alpha;
mod builder;
mod error;
mod geometry;
mod mesh;
mod polygonize;
mod preprocess;
mod triangulation;
mod types;

pub use alpha::{alpha_shape, alpha_shape_auto, estimate_alpha_radius};
pub use builder::{
    HullMode, IsoHull, IsoHullAreaBuilder, IsoHullBuildBuilder, IsoHullInputBuilder,
};
pub use error::{AlphaShapeError, IsoHullError};
pub use types::{LatLon, MultiPolygon, Point2, Polygon};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_alpha_shape() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];
        let shape = alpha_shape(points, 1.0).unwrap();

        assert_eq!(shape.polygons.len(), 1);
        assert_eq!(
            shape.polygons[0].outer.first(),
            shape.polygons[0].outer.last()
        );
    }

    #[test]
    fn square_auto_alpha_shape() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];

        let alpha_radius = estimate_alpha_radius(points.clone()).unwrap();
        let shape = alpha_shape_auto(points).unwrap();

        assert!(alpha_radius > 0.0);
        assert_eq!(shape.polygons.len(), 1);
    }

    #[test]
    fn builder_keeps_step_order_explicit() {
        let points = vec![
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
            Point2::new(0.0, 1.0),
        ];

        let shape = IsoHull::from_xy(points)
            .auto_alpha()
            .all_area()
            .build()
            .unwrap();

        assert_eq!(shape.polygons.len(), 1);
    }

    #[test]
    fn builder_subsample_mode_is_opt_in() {
        let mut points = Vec::new();
        for y in 0..100 {
            for x in 0..100 {
                points.push(Point2::new(x as f64, y as f64));
            }
        }

        let shape = IsoHull::from_xy(points)
            .mode(HullMode::Subsample { max_points: 10_000 })
            .auto_alpha()
            .all_area()
            .build()
            .unwrap();

        assert_eq!(shape.polygons.len(), 1);
    }
}
