#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    pub x: f64,
    pub y: f64,
}

impl Point2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

impl From<(f64, f64)> for Point2 {
    fn from((x, y): (f64, f64)) -> Self {
        Self::new(x, y)
    }
}

impl From<&Point2> for Point2 {
    fn from(point: &Point2) -> Self {
        *point
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

impl LatLon {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }
}

impl From<(f64, f64)> for LatLon {
    fn from((lat, lon): (f64, f64)) -> Self {
        Self::new(lat, lon)
    }
}

impl From<&LatLon> for LatLon {
    fn from(point: &LatLon) -> Self {
        *point
    }
}

#[derive(Debug, Clone)]
pub struct Polygon {
    pub outer: Vec<Point2>,
}

#[derive(Debug, Clone)]
pub struct MultiPolygon {
    pub polygons: Vec<Polygon>,
}
