# IsoHull

IsoHull builds solid alpha-shape hulls from projected 2D points or latitude/longitude samples.

The algorithm always computes the hull in a local projected XY space so distances, areas, and alpha radii are meaningful. When you start from latitude/longitude input, IsoHull projects internally, builds the hull, then converts the final polygon vertices back to latitude/longitude.

## Install

```toml
[dependencies]
isohull = "0.1"
```

GeoJSON export is optional:

```toml
[dependencies]
isohull = { version = "0.1", features = ["geojson"] }
```

## Builder Flow

The builder is intentionally step-by-step:

1. Input: `IsoHull::from_xy(...)` or `IsoHull::from_lat_lon(...)`.
2. Optional speed/quality mode: `.mode(HullMode::...)`.
3. Alpha: `.auto_alpha()` or `.alpha(radius)`.
4. Area filtering: `.min_area_ratio(ratio)` or `.all_area()`.
5. Output: `.build()`.

`HullMode::Exact` is the default.

## Projected XY Input

Use `from_xy` when your input is already projected planar coordinates.

```rust
use isohull::{IsoHull, Point2};

let points = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(1.0, 1.0),
    Point2::new(0.0, 1.0),
];

let shape = IsoHull::from_xy(points)
    .auto_alpha()
    .min_area_ratio(0.005)
    .build()?;

# Ok::<(), isohull::IsoHullError>(())
```

`from_xy(...).build()` returns `MultiPolygon`, whose coordinates are projected XY coordinates.

## Latitude/Longitude Input

Use `from_lat_lon` when your input is geographic coordinates.

```rust
use isohull::{HullMode, IsoHull, LatLon};

let points = vec![
    LatLon::new(50.0, 3.0),
    LatLon::new(50.0, 3.01),
    LatLon::new(50.01, 3.01),
    LatLon::new(50.01, 3.0),
];

let shape = IsoHull::from_lat_lon(points)
    .mode(HullMode::Low)
    .auto_alpha()
    .all_area()
    .build()?;

# Ok::<(), isohull::IsoHullError>(())
```

`from_lat_lon(...).build()` returns `GeoMultiPolygon`, whose coordinates are latitude/longitude coordinates.

## Speed/Quality Modes

Exact mode uses every input point. The other modes spatially subsample the input before triangulation, which is much faster on large point clouds and intentionally less exact.

- `HullMode::Low`: up to 10,000 sampled points
- `HullMode::Medium`: up to 50,000 sampled points
- `HullMode::High`: up to 100,000 sampled points
- `HullMode::Ultra`: up to 250,000 sampled points
- `HullMode::Exact`: no subsampling

```rust
use isohull::{HullMode, IsoHull};

let shape = IsoHull::from_xy(points)
    .mode(HullMode::Medium)
    .auto_alpha()
    .min_area_ratio(0.005)
    .build()?;

# Ok::<(), isohull::IsoHullError>(())
```

## Alpha Selection

Use `.auto_alpha()` for the built-in alpha estimate:

```rust
let shape = IsoHull::from_xy(points)
    .auto_alpha()
    .all_area()
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

Use `.alpha(radius)` when you want to provide the alpha radius yourself:

```rust
let shape = IsoHull::from_xy(points)
    .alpha(250.0)
    .all_area()
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

For `from_xy`, the radius is in your input coordinate units. For `from_lat_lon`, the radius is in projected meters.

## Area Filtering

Use `.all_area()` to keep every returned polygon.

Use `.min_area_ratio(ratio)` to remove tiny disconnected regions. It keeps polygons whose area is at least:

```text
ratio * largest_polygon_area
```

For example:

```rust
let shape = IsoHull::from_xy(points)
    .auto_alpha()
    .min_area_ratio(0.005)
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

## GeoJSON

GeoJSON export is available only for latitude/longitude output and only when the `geojson` feature is enabled.

```toml
[dependencies]
isohull = { version = "0.1", features = ["geojson"] }
```

```rust
use isohull::{HullMode, IsoHull, LatLon};

let geojson = IsoHull::from_lat_lon(vec![
    LatLon::new(50.0, 3.0),
    LatLon::new(50.0, 3.01),
    LatLon::new(50.01, 3.01),
    LatLon::new(50.01, 3.0),
])
.mode(HullMode::Low)
.auto_alpha()
.all_area()
.build()?
.to_geojson()
.to_string();

# Ok::<(), isohull::IsoHullError>(())
```

Without the `geojson` feature, `to_geojson()` is not compiled and cannot be called.

## Lower-Level Helpers

The lower-level projected XY helpers remain available:

- `alpha_shape(points, alpha_radius)`
- `alpha_shape_auto(points)`
- `estimate_alpha_radius(points)`

These operate on `Point2` and return projected `MultiPolygon` values.

## Tracing

IsoHull uses the `log` crate. To inspect timing for every build step, run:

```sh
RUST_LOG=isohull=trace cargo run --release --example trace_examples
```

The trace example defaults to `RUST_LOG=isohull=trace` when `RUST_LOG` is unset.

To trace a specific hull mode:

```sh
ISOHULL_MODE=low RUST_LOG=isohull=trace cargo run --release --example trace_examples
```

Supported `ISOHULL_MODE` values are `low`, `medium`, `high`, `ultra`, and `exact`.

You can also trace a specific target:

```sh
RUST_LOG=isohull::triangulation=trace cargo run --release --example trace_examples
```

## Benchmarks

Benchmark every example dataset against every hull mode with:

```sh
cargo bench --bench examples
```

## Release

Publishing is automated through GitHub Actions.

One repository secret is required:

```text
CARGO_API_KEY
```

It must contain a crates.io API token allowed to publish `isohull`.

To cut a release from a clean working tree:

```sh
./release.sh 0.1.1
```

The script runs formatting, clippy, tests, benchmark build checks, updates `Cargo.toml`, commits `Release v0.1.1`, creates the `v0.1.1` tag, and pushes both the branch and tag.

Pushing a `v*.*.*` tag triggers `.github/workflows/publish-crate.yml`, which verifies the crate with `cargo publish --dry-run`, publishes to crates.io, then creates or updates the matching GitHub release.

The workflow can also be started manually from GitHub Actions. Manual runs always perform the checks and dry-run; set the `publish` input to `true` only when you intentionally want to publish from that run.
