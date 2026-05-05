use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use isohull::{HullMode, IsoHull, LatLon};
use serde::Deserialize;

const LARGE_POINT_COUNT: usize = 1_400_000;

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

#[derive(Debug, Clone)]
struct ExampleCase {
    name: String,
    points: Vec<LatLon>,
}

fn bench_examples(criterion: &mut Criterion) {
    let examples = load_examples();
    let medium = medium_example(&examples);
    let large = large_example(&examples, &medium);
    let cases = [
        ExampleCase {
            name: format!("65k/{}", medium.name),
            points: medium.points,
        },
        large,
    ];
    let modes = [
        ModeCase {
            name: "exact",
            mode: HullMode::Exact,
        },
        ModeCase {
            name: "subsample_10k",
            mode: HullMode::Subsample { max_points: 10_000 },
        },
        ModeCase {
            name: "subsample_50k",
            mode: HullMode::Subsample { max_points: 50_000 },
        },
    ];

    let mut group = criterion.benchmark_group("hull_modes");
    group.sample_size(10);
    group.warm_up_time(Duration::from_secs(1));
    group.measurement_time(Duration::from_secs(10));

    for case in &cases {
        for mode in modes {
            let id = BenchmarkId::new(format!("{}/{}", case.name, mode.name), case.points.len());

            group.bench_with_input(id, &case.points, |bencher, points| {
                bencher.iter(|| {
                    black_box(
                        IsoHull::from_lat_lon(points.iter())
                            .mode(mode.mode)
                            .auto_alpha()
                            .min_area_ratio(0.005)
                            .build()
                            .expect("example should build"),
                    )
                });
            });
        }
    }

    group.finish();
}

#[derive(Debug, Clone, Copy)]
struct ModeCase {
    name: &'static str,
    mode: HullMode,
}

fn medium_example(examples: &[ExampleCase]) -> ExampleCase {
    examples
        .iter()
        .filter(|example| example.points.len() <= 100_000)
        .max_by_key(|example| example.points.len())
        .cloned()
        .expect("at least one medium example should exist")
}

fn large_example(examples: &[ExampleCase], medium: &ExampleCase) -> ExampleCase {
    if let Some(example) = examples
        .iter()
        .filter(|example| example.points.len() >= 1_000_000)
        .max_by_key(|example| example.points.len())
    {
        let points = example
            .points
            .iter()
            .copied()
            .take(LARGE_POINT_COUNT)
            .collect::<Vec<_>>();

        return ExampleCase {
            name: format!("1_4m/{}", example.name),
            points,
        };
    }

    ExampleCase {
        name: "1_4m/synthetic_from_65k".to_string(),
        points: expand_points(&medium.points, LARGE_POINT_COUNT),
    }
}

fn expand_points(source: &[LatLon], target_len: usize) -> Vec<LatLon> {
    let bounds = LatLonBounds::from_points(source);
    let lat_step = (bounds.max_lat - bounds.min_lat).abs().max(0.01) * 1.5;
    let lon_step = (bounds.max_lon - bounds.min_lon).abs().max(0.01) * 1.5;
    let tiles = target_len.div_ceil(source.len());
    let columns = (tiles as f64).sqrt().ceil() as usize;

    let mut points = Vec::with_capacity(target_len);
    for tile in 0..tiles {
        let row = tile / columns;
        let column = tile % columns;
        let lat_offset = row as f64 * lat_step;
        let lon_offset = column as f64 * lon_step;

        for point in source {
            if points.len() == target_len {
                return points;
            }

            points.push(LatLon::new(point.lat + lat_offset, point.lon + lon_offset));
        }
    }

    points
}

#[derive(Debug, Clone, Copy)]
struct LatLonBounds {
    min_lat: f64,
    max_lat: f64,
    min_lon: f64,
    max_lon: f64,
}

impl LatLonBounds {
    fn from_points(points: &[LatLon]) -> Self {
        let first = points[0];
        let mut bounds = Self {
            min_lat: first.lat,
            max_lat: first.lat,
            min_lon: first.lon,
            max_lon: first.lon,
        };

        for point in &points[1..] {
            bounds.min_lat = bounds.min_lat.min(point.lat);
            bounds.max_lat = bounds.max_lat.max(point.lat);
            bounds.min_lon = bounds.min_lon.min(point.lon);
            bounds.max_lon = bounds.max_lon.max(point.lon);
        }

        bounds
    }
}

fn load_examples() -> Vec<ExampleCase> {
    let input_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/examples");

    json_files(&input_dir)
        .into_iter()
        .map(|path| {
            let example = read_example(&path);
            assert_eq!(example.point_count, example.points.len());

            ExampleCase {
                name: file_stem(&path).to_string(),
                points: example
                    .points
                    .into_iter()
                    .map(|point| LatLon::new(point.lat, point.lon))
                    .collect(),
            }
        })
        .collect()
}

fn json_files(input_dir: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(input_dir)
        .expect("data/examples should exist")
        .map(|entry| entry.expect("example entry should be readable").path())
        .collect::<Vec<_>>();

    paths.retain(|path| path.extension().and_then(|extension| extension.to_str()) == Some("json"));
    paths.sort();

    paths
}

fn read_example(path: &Path) -> Example {
    let contents = fs::read_to_string(path).expect("example should be readable");
    serde_json::from_str(&contents).expect("example should be valid JSON")
}

fn file_stem(path: &Path) -> &str {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .expect("example should have UTF-8 file stem")
}

criterion_group!(benches, bench_examples);
criterion_main!(benches);
