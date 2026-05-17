use std::{
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use isohull::{GeoMultiPolygon, HullMode, IsoHull, LatLon, Point2};
use serde::Deserialize;

const SVG_WIDTH: f64 = 1200.0;
const SVG_HEIGHT: f64 = 1200.0;
const SVG_PADDING: f64 = 36.0;

#[derive(Debug, Deserialize)]
struct Example {
    point_count: usize,
    points: Vec<GeoPoint>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
struct GeoPoint {
    lat: f64,
    lon: f64,
}

#[test]
fn render_example_alpha_shapes_to_svg_exact() -> Result<(), Box<dyn Error>> {
    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/isohull/examples/exact");

    fs::create_dir_all(&output_dir)?;

    for path in json_files(&input_dir)? {
        let example = read_example(&path)?;
        assert_eq!(example.point_count, example.points.len());

        let shape = IsoHull::from_lat_lon(
            example
                .points
                .iter()
                .map(|point| LatLon::new(point.lat, point.lon)),
        )
        .auto_alpha()
        .min_area_ratio(0.005)
        .build()?;
        let svg = render_svg(&shape);

        fs::write(output_dir.join(format!("{}.svg", file_stem(&path)?)), svg)?;
    }

    Ok(())
}

#[test]
fn render_example_alpha_shapes_to_svg_low() -> Result<(), Box<dyn Error>> {
    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/isohull/examples/low");

    fs::create_dir_all(&output_dir)?;

    for path in json_files(&input_dir)? {
        let example = read_example(&path)?;
        assert_eq!(example.point_count, example.points.len());

        let shape = IsoHull::from_lat_lon(
            example
                .points
                .iter()
                .map(|point| LatLon::new(point.lat, point.lon)),
        )
        .mode(HullMode::Low)
        .auto_alpha()
        .min_area_ratio(0.005)
        .build()?;
        let svg = render_svg(&shape);

        fs::write(output_dir.join(format!("{}.svg", file_stem(&path)?)), svg)?;
    }

    Ok(())
}

#[test]
fn render_example_alpha_shapes_to_svg_medium() -> Result<(), Box<dyn Error>> {
    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/isohull/examples/medium");

    fs::create_dir_all(&output_dir)?;

    for path in json_files(&input_dir)? {
        let example = read_example(&path)?;
        assert_eq!(example.point_count, example.points.len());

        let shape = IsoHull::from_lat_lon(
            example
                .points
                .iter()
                .map(|point| LatLon::new(point.lat, point.lon)),
        )
        .mode(HullMode::Medium)
        .auto_alpha()
        .min_area_ratio(0.005)
        .build()?;
        let svg = render_svg(&shape);

        fs::write(output_dir.join(format!("{}.svg", file_stem(&path)?)), svg)?;
    }

    Ok(())
}

#[test]
fn render_example_alpha_shapes_to_svg_high() -> Result<(), Box<dyn Error>> {
    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");
    let output_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/isohull/examples/high");

    fs::create_dir_all(&output_dir)?;

    for path in json_files(&input_dir)? {
        let example = read_example(&path)?;
        assert_eq!(example.point_count, example.points.len());

        let shape = IsoHull::from_lat_lon(
            example
                .points
                .iter()
                .map(|point| LatLon::new(point.lat, point.lon)),
        )
        .mode(HullMode::High)
        .auto_alpha()
        .min_area_ratio(0.005)
        .build()?;
        let svg = render_svg(&shape);

        fs::write(output_dir.join(format!("{}.svg", file_stem(&path)?)), svg)?;
    }

    Ok(())
}

fn json_files(input_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut paths = fs::read_dir(input_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?;

    paths.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"));
    paths.sort();

    Ok(paths)
}

fn read_example(path: &Path) -> Result<Example, Box<dyn Error>> {
    let contents = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&contents)?)
}

fn render_svg(shape: &GeoMultiPolygon) -> String {
    let bounds = Bounds::from_shape(shape);
    let projector = SvgProjector::new(bounds);
    let paths = shape
        .polygons
        .iter()
        .map(|polygon| {
            format!(
                "<path d=\"{}\" fill=\"#2563eb\" fill-opacity=\"0.22\" stroke=\"#1d4ed8\" stroke-width=\"2\" />",
                path_data(&polygon.outer, projector)
            )
        })
        .collect::<Vec<_>>()
        .join("\n  ");

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{SVG_WIDTH}\" height=\"{SVG_HEIGHT}\" viewBox=\"0 0 {SVG_WIDTH} {SVG_HEIGHT}\">
  <rect width=\"100%\" height=\"100%\" fill=\"#f8fafc\" />
  {paths}
</svg>"
    )
}

fn path_data(points: &[LatLon], projector: SvgProjector) -> String {
    let mut data = String::new();

    for (index, point) in points.iter().enumerate() {
        let point = projector.point(*point);
        let command = if index == 0 { 'M' } else { 'L' };
        data.push_str(&format!("{command}{:.2},{:.2}", point.x, point.y));
    }

    data.push('Z');
    data
}

fn file_stem(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("could not read file stem for {}", path.display()).into())
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn from_shape(shape: &GeoMultiPolygon) -> Self {
        shape
            .polygons
            .iter()
            .flat_map(|polygon| polygon.outer.iter())
            .fold(
                Self {
                    min_x: f64::INFINITY,
                    min_y: f64::INFINITY,
                    max_x: f64::NEG_INFINITY,
                    max_y: f64::NEG_INFINITY,
                },
                |bounds, point| Self {
                    min_x: bounds.min_x.min(point.lon),
                    min_y: bounds.min_y.min(point.lat),
                    max_x: bounds.max_x.max(point.lon),
                    max_y: bounds.max_y.max(point.lat),
                },
            )
    }

    fn width(self) -> f64 {
        (self.max_x - self.min_x).max(1.0)
    }

    fn height(self) -> f64 {
        (self.max_y - self.min_y).max(1.0)
    }
}

#[derive(Debug, Clone, Copy)]
struct SvgProjector {
    bounds: Bounds,
    scale: f64,
}

impl SvgProjector {
    fn new(bounds: Bounds) -> Self {
        let content_width = SVG_WIDTH - SVG_PADDING * 2.0;
        let content_height = SVG_HEIGHT - SVG_PADDING * 2.0;
        let scale = (content_width / bounds.width()).min(content_height / bounds.height());

        Self { bounds, scale }
    }

    fn point(self, point: LatLon) -> Point2 {
        let x = SVG_PADDING + (point.lon - self.bounds.min_x) * self.scale;
        let y = SVG_HEIGHT - SVG_PADDING - (point.lat - self.bounds.min_y) * self.scale;
        Point2::new(x, y)
    }
}
