# IsoHull

IsoHull builds solid alpha-shape polygons from 2D point clouds or latitude/longitude samples.

```rust
use isohull::{IsoHull, Point2};

let points = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(1.0, 1.0),
    Point2::new(0.0, 1.0),
];

let multipolygon = IsoHull::from_xy(points)
    .auto_alpha()
    .min_area_ratio(0.005)
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

The builder is intentionally step-by-step:

1. Input: `IsoHull::from_xy(...)` or `IsoHull::from_lat_lon(...)`.
2. Optional mode: `.mode(HullMode::...)`.
3. Alpha: `.auto_alpha()` or `.alpha(radius)`.
4. Area filtering: `.min_area_ratio(ratio)` or `.all_area()`.
5. Output: `.build()`.

`min_area_ratio` only applies when multiple polygons are returned. It keeps polygons whose area is at least `ratio * largest_polygon_area`.

Exact mode is the default. For large clouds where visualization speed matters more than exactness, opt into grid subsampling before alpha selection:

```rust
use isohull::HullMode;

let multipolygon = IsoHull::from_xy(points)
    .mode(HullMode::Subsample { max_points: 10_000 })
    .auto_alpha()
    .min_area_ratio(0.005)
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

The lower-level helpers remain available:

- `alpha_shape(points, alpha_radius)`
- `alpha_shape_auto(points)`
- `estimate_alpha_radius(points)`

Benchmark the example datasets with:

```sh
cargo bench --bench examples
```

Inspect trace timings for every build step with:

```sh
RUST_LOG=isohull=trace cargo run --release --example trace_examples
```

Trace the subsampling path with:

```sh
ISOHULL_SUBSAMPLE_MAX_POINTS=10000 RUST_LOG=isohull=trace cargo run --release --example trace_examples
```

The example defaults to `RUST_LOG=isohull=trace` when `RUST_LOG` is unset. Override it when needed:

```sh
RUST_LOG=isohull::triangulation=trace cargo run --release --example trace_examples
```
