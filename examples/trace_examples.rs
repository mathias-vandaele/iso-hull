use std::{
    env,
    error::Error,
    fs,
    path::{Path, PathBuf},
};

use isohull::{HullMode, IsoHull, LatLon};
use serde::Deserialize;

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

fn main() -> Result<(), Box<dyn Error>> {
    init_logger();

    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");
    let mode = hull_mode()?;

    for path in json_files(&input_dir)? {
        let example = read_example(&path)?;
        assert_eq!(example.point_count, example.points.len());

        log::info!(
            target: "isohull::example",
            "building example file={} points={}",
            file_stem(&path)?,
            example.points.len()
        );

        let builder = IsoHull::from_lat_lon(
            example
                .points
                .iter()
                .map(|point| LatLon::new(point.lat, point.lon)),
        );
        let builder = if let Some(mode) = mode {
            builder.mode(mode)
        } else {
            builder
        };

        let shape = builder.auto_alpha().min_area_ratio(0.005).build()?;

        log::info!(
            target: "isohull::example",
            "built example file={} polygons={}",
            file_stem(&path)?,
            shape.polygons.len()
        );
    }

    Ok(())
}

fn hull_mode() -> Result<Option<HullMode>, Box<dyn Error>> {
    let Ok(value) = env::var("ISOHULL_MODE") else {
        return Ok(None);
    };

    match value.to_ascii_lowercase().as_str() {
        "low" => Ok(Some(HullMode::Low)),
        "medium" => Ok(Some(HullMode::Medium)),
        "high" => Ok(Some(HullMode::High)),
        "ultra" => Ok(Some(HullMode::Ultra)),
        "exact" => Ok(Some(HullMode::Exact)),
        _ => Err(format!("unknown ISOHULL_MODE: {value}").into()),
    }
}

fn init_logger() {
    let env = env_logger::Env::default().default_filter_or("isohull=trace");

    env_logger::Builder::from_env(env)
        .format_timestamp_millis()
        .init();
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

fn file_stem(path: &Path) -> Result<&str, Box<dyn Error>> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| format!("could not read file stem for {}", path.display()).into())
}
