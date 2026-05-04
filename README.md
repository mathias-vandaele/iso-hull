# IsoHull

IsoHull reconstructs solid `MultiPolygon` shapes from 2D point clouds that represent isochrone-like reachable areas.

```rust
use isohull::{IsoHull, Point2};

let points = vec![
    Point2::new(0.0, 0.0),
    Point2::new(1.0, 0.0),
    Point2::new(1.0, 1.0),
    Point2::new(0.0, 1.0),
];

let multipolygon = IsoHull::from_xy(points)
    .auto_scale()
    .remove_bridges()
    .min_area_ratio(0.005)
    .build()?;
# Ok::<(), isohull::IsoHullError>(())
```

IsoHull has three user decisions:

1. Scale: use `auto_scale()` or provide `manual_scale().alpha_radius(...).max_edge_length(...)`.
2. Connectivity: use `remove_bridges()` or `no_bridge_removal()`.
3. Area filtering: use `min_area_ratio(...)` or `no_area_filter()`.

IsoHull produces solid polygons only. Holes are intentionally not represented in this first version.
