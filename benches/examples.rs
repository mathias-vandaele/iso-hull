use std::{
    fs,
    path::{Path, PathBuf},
};

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
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

#[derive(Debug)]
struct ExampleCase {
    name: String,
    points: Vec<LatLon>,
}

fn bench_examples(criterion: &mut Criterion) {
    let examples = load_examples();
    let modes = [
        ModeCase {
            name: "low",
            mode: HullMode::Low,
        },
        ModeCase {
            name: "medium",
            mode: HullMode::Medium,
        },
        ModeCase {
            name: "high",
            mode: HullMode::High,
        },
        ModeCase {
            name: "ultra",
            mode: HullMode::Ultra,
        },
        ModeCase {
            name: "exact",
            mode: HullMode::Exact,
        },
    ];

    let mut group = criterion.benchmark_group("hull_modes");

    for example in &examples {
        for mode in modes {
            let id = BenchmarkId::new(
                format!("{}/{}", example.name, mode.name),
                example.points.len(),
            );

            group.bench_with_input(id, &example.points, |bencher, points| {
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
