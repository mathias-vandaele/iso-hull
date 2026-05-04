use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    fmt,
};

use spade::{DelaunayTriangulation, HasPosition, Triangulation};

const AREA_EPSILON: f64 = 1.0e-12;

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

#[derive(Debug, Clone)]
pub struct Polygon {
    pub outer: Vec<Point2>,
}

#[derive(Debug, Clone)]
pub struct MultiPolygon {
    pub polygons: Vec<Polygon>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlphaShapeError {
    NotEnoughPoints(usize),
    InvalidAlpha,
    InvalidPoint,
    TriangulationFailed(String),
    EmptyShape,
}

impl fmt::Display for AlphaShapeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotEnoughPoints(count) => {
                write!(
                    formatter,
                    "not enough unique points: expected 3, got {count}"
                )
            }
            Self::InvalidAlpha => write!(formatter, "alpha radius must be finite and positive"),
            Self::InvalidPoint => write!(formatter, "point coordinates must be finite"),
            Self::TriangulationFailed(error) => write!(formatter, "triangulation failed: {error}"),
            Self::EmptyShape => write!(formatter, "alpha shape is empty"),
        }
    }
}

impl Error for AlphaShapeError {}

pub fn alpha_shape(
    points: impl Into<Vec<Point2>>,
    alpha_radius: f64,
) -> Result<MultiPolygon, AlphaShapeError> {
    if !alpha_radius.is_finite() || alpha_radius <= 0.0 {
        return Err(AlphaShapeError::InvalidAlpha);
    }

    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;
    build_alpha_shape(&points, &triangles, alpha_radius)
}

pub fn alpha_shape_auto(points: impl Into<Vec<Point2>>) -> Result<MultiPolygon, AlphaShapeError> {
    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;
    let alpha_radius = estimate_alpha_radius_from_triangles(&points, &triangles)?;

    build_alpha_shape(&points, &triangles, alpha_radius)
}

pub fn estimate_alpha_radius(points: impl Into<Vec<Point2>>) -> Result<f64, AlphaShapeError> {
    let points = prepare_points(points.into())?;
    let triangles = delaunay_triangles(&points)?;

    estimate_alpha_radius_from_triangles(&points, &triangles)
}

fn build_alpha_shape(
    points: &[Point2],
    triangles: &[Triangle],
    alpha_radius: f64,
) -> Result<MultiPolygon, AlphaShapeError> {
    let kept_triangles = triangles
        .iter()
        .copied()
        .filter(|triangle| {
            triangle_circumradius(*triangle, points).is_some_and(|radius| radius <= alpha_radius)
        })
        .collect::<Vec<_>>();

    if kept_triangles.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    let polygons = triangle_components(&kept_triangles)
        .into_iter()
        .filter_map(|component| polygon_from_component(&component, points))
        .collect::<Vec<_>>();

    if polygons.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    Ok(MultiPolygon { polygons })
}

fn estimate_alpha_radius_from_triangles(
    points: &[Point2],
    triangles: &[Triangle],
) -> Result<f64, AlphaShapeError> {
    let mut radii = triangles
        .iter()
        .filter_map(|triangle| triangle_circumradius(*triangle, points))
        .filter(|radius| radius.is_finite() && *radius > 0.0)
        .collect::<Vec<_>>();

    if radii.is_empty() {
        return Err(AlphaShapeError::EmptyShape);
    }

    radii.sort_by(f64::total_cmp);

    let reference_alpha = percentile_sorted(&radii, 0.995).unwrap_or(radii[radii.len() - 1]);
    let reference_shape = build_alpha_shape(points, triangles, reference_alpha)
        .or_else(|_| build_alpha_shape(points, triangles, radii[radii.len() - 1]))?;
    let reference_area = total_area(&reference_shape);
    let target_polygon_count = reference_shape.polygons.len().max(1);

    let candidate_percentiles = [
        0.50, 0.60, 0.70, 0.80, 0.85, 0.90, 0.93, 0.95, 0.97, 0.98, 0.99, 0.995,
    ];

    let mut last_alpha = None;
    for percentile in candidate_percentiles {
        let Some(alpha) = percentile_sorted(&radii, percentile) else {
            continue;
        };

        if last_alpha == Some(alpha) {
            continue;
        }
        last_alpha = Some(alpha);

        let Ok(shape) = build_alpha_shape(points, triangles, alpha) else {
            continue;
        };

        if shape.polygons.len() <= target_polygon_count
            && total_area(&shape) >= reference_area * 0.85
        {
            return Ok(alpha);
        }
    }

    Ok(reference_alpha)
}

fn percentile_sorted(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let percentile = percentile.clamp(0.0, 1.0);
    let index = (percentile * (values.len() - 1) as f64).ceil() as usize;
    Some(values[index])
}

fn total_area(shape: &MultiPolygon) -> f64 {
    shape
        .polygons
        .iter()
        .map(|polygon| signed_area(&polygon.outer).abs())
        .sum()
}

fn prepare_points(points: Vec<Point2>) -> Result<Vec<Point2>, AlphaShapeError> {
    let mut unique_points = Vec::new();
    let mut seen = HashSet::new();

    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(AlphaShapeError::InvalidPoint);
        }

        if seen.insert((point.x.to_bits(), point.y.to_bits())) {
            unique_points.push(point);
        }
    }

    if unique_points.len() < 3 {
        return Err(AlphaShapeError::NotEnoughPoints(unique_points.len()));
    }

    Ok(unique_points)
}

#[derive(Debug, Clone, Copy)]
struct IndexedPoint {
    index: usize,
    point: Point2,
}

impl HasPosition for IndexedPoint {
    type Scalar = f64;

    fn position(&self) -> spade::Point2<Self::Scalar> {
        spade::Point2::new(self.point.x, self.point.y)
    }
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: usize,
    b: usize,
    c: usize,
}

impl Triangle {
    fn edges(self) -> [Edge; 3] {
        [
            Edge::new(self.a, self.b),
            Edge::new(self.b, self.c),
            Edge::new(self.c, self.a),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Edge {
    u: usize,
    v: usize,
}

impl Edge {
    fn new(a: usize, b: usize) -> Self {
        if a <= b {
            Self { u: a, v: b }
        } else {
            Self { u: b, v: a }
        }
    }
}

fn delaunay_triangles(points: &[Point2]) -> Result<Vec<Triangle>, AlphaShapeError> {
    let vertices = points
        .iter()
        .enumerate()
        .map(|(index, point)| IndexedPoint {
            index,
            point: *point,
        })
        .collect::<Vec<_>>();

    let triangulation = DelaunayTriangulation::<IndexedPoint>::bulk_load_stable(vertices)
        .map_err(|error| AlphaShapeError::TriangulationFailed(error.to_string()))?;

    Ok(triangulation
        .inner_faces()
        .map(|face| {
            let vertices = face.vertices();
            Triangle {
                a: vertices[0].data().index,
                b: vertices[1].data().index,
                c: vertices[2].data().index,
            }
        })
        .collect())
}

fn triangle_circumradius(triangle: Triangle, points: &[Point2]) -> Option<f64> {
    circumradius(points[triangle.a], points[triangle.b], points[triangle.c])
}

fn circumradius(a: Point2, b: Point2, c: Point2) -> Option<f64> {
    let ab = distance(a, b);
    let bc = distance(b, c);
    let ca = distance(c, a);
    let area = ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs() * 0.5;

    if area <= AREA_EPSILON {
        None
    } else {
        Some(ab * bc * ca / (4.0 * area))
    }
}

fn distance(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

fn triangle_components(triangles: &[Triangle]) -> Vec<Vec<Triangle>> {
    let mut triangles_by_edge: HashMap<Edge, Vec<usize>> = HashMap::new();

    for (index, triangle) in triangles.iter().enumerate() {
        for edge in triangle.edges() {
            triangles_by_edge.entry(edge).or_default().push(index);
        }
    }

    let mut components = Vec::new();
    let mut visited = vec![false; triangles.len()];

    for start in 0..triangles.len() {
        if visited[start] {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        visited[start] = true;

        while let Some(index) = queue.pop_front() {
            let triangle = triangles[index];
            component.push(triangle);

            for edge in triangle.edges() {
                if let Some(neighbors) = triangles_by_edge.get(&edge) {
                    for &neighbor in neighbors {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        components.push(component);
    }

    components
}

fn polygon_from_component(component: &[Triangle], points: &[Point2]) -> Option<Polygon> {
    let boundary = boundary_edges(component);
    let mut largest_ring = extract_rings(&boundary, points)
        .into_iter()
        .max_by(|a, b| signed_area(a).abs().total_cmp(&signed_area(b).abs()))?;

    close_ring(&mut largest_ring);
    ensure_counterclockwise(&mut largest_ring);

    is_valid_ring(&largest_ring).then_some(Polygon {
        outer: largest_ring,
    })
}

fn boundary_edges(triangles: &[Triangle]) -> Vec<Edge> {
    let mut counts = HashMap::new();

    for triangle in triangles {
        for edge in triangle.edges() {
            *counts.entry(edge).or_insert(0usize) += 1;
        }
    }

    counts
        .into_iter()
        .filter_map(|(edge, count)| (count == 1).then_some(edge))
        .collect()
}

fn extract_rings(edges: &[Edge], points: &[Point2]) -> Vec<Vec<Point2>> {
    edge_components(&prune_dangling_edges(edges))
        .into_iter()
        .filter_map(|component| simple_cycle_walk(&component, points))
        .filter(|ring| is_valid_ring(ring))
        .collect()
}

fn prune_dangling_edges(edges: &[Edge]) -> Vec<Edge> {
    let mut active_edges = edges.iter().copied().collect::<HashSet<_>>();

    loop {
        let degrees = vertex_degrees(&active_edges);
        let dangling_vertices = degrees
            .into_iter()
            .filter_map(|(vertex, degree)| (degree < 2).then_some(vertex))
            .collect::<HashSet<_>>();

        if dangling_vertices.is_empty() {
            break;
        }

        let before = active_edges.len();
        active_edges.retain(|edge| {
            !dangling_vertices.contains(&edge.u) && !dangling_vertices.contains(&edge.v)
        });

        if active_edges.len() == before {
            break;
        }
    }

    let mut edges = active_edges.into_iter().collect::<Vec<_>>();
    edges.sort_by_key(|edge| (edge.u, edge.v));
    edges
}

fn vertex_degrees(edges: &HashSet<Edge>) -> HashMap<usize, usize> {
    let mut degrees = HashMap::new();

    for edge in edges {
        *degrees.entry(edge.u).or_insert(0) += 1;
        *degrees.entry(edge.v).or_insert(0) += 1;
    }

    degrees
}

fn edge_components(edges: &[Edge]) -> Vec<Vec<Edge>> {
    let mut edges_by_vertex: HashMap<usize, Vec<usize>> = HashMap::new();

    for (index, edge) in edges.iter().enumerate() {
        edges_by_vertex.entry(edge.u).or_default().push(index);
        edges_by_vertex.entry(edge.v).or_default().push(index);
    }

    let mut components = Vec::new();
    let mut visited = vec![false; edges.len()];

    for start in 0..edges.len() {
        if visited[start] {
            continue;
        }

        let mut component = Vec::new();
        let mut queue = VecDeque::from([start]);
        visited[start] = true;

        while let Some(index) = queue.pop_front() {
            let edge = edges[index];
            component.push(edge);

            for vertex in [edge.u, edge.v] {
                if let Some(neighbors) = edges_by_vertex.get(&vertex) {
                    for &neighbor in neighbors {
                        if !visited[neighbor] {
                            visited[neighbor] = true;
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
        }

        components.push(component);
    }

    components
}

fn simple_cycle_walk(edges: &[Edge], points: &[Point2]) -> Option<Vec<Point2>> {
    let mut adjacency: HashMap<usize, Vec<usize>> = HashMap::new();

    for edge in edges {
        adjacency.entry(edge.u).or_default().push(edge.v);
        adjacency.entry(edge.v).or_default().push(edge.u);
    }

    for neighbors in adjacency.values_mut() {
        neighbors.sort_by(|a, b| compare_points(points[*a], points[*b]));
        neighbors.dedup();

        if neighbors.len() != 2 {
            return None;
        }
    }

    let start = adjacency
        .keys()
        .copied()
        .min_by(|a, b| compare_points(points[*a], points[*b]))?;
    let mut previous = start;
    let mut current = *adjacency.get(&start)?.first()?;
    let mut ring_indices = vec![start];

    for _ in 0..=edges.len() {
        ring_indices.push(current);

        if current == start {
            break;
        }

        let next = adjacency
            .get(&current)?
            .iter()
            .copied()
            .find(|&next| next != previous)?;
        previous = current;
        current = next;
    }

    if ring_indices.last().copied() != Some(start) {
        return None;
    }

    if ring_indices.len() != edges.len() + 1 {
        return None;
    }

    let unique_vertices = ring_indices[..ring_indices.len() - 1]
        .iter()
        .copied()
        .collect::<HashSet<_>>();
    if unique_vertices.len() != ring_indices.len() - 1 {
        return None;
    }

    Some(
        ring_indices
            .into_iter()
            .map(|index| points[index])
            .collect(),
    )
}

fn close_ring(points: &mut Vec<Point2>) {
    let Some(first) = points.first().copied() else {
        return;
    };

    if points.last().copied() != Some(first) {
        points.push(first);
    }
}

fn ensure_counterclockwise(points: &mut Vec<Point2>) {
    if signed_area(points) >= 0.0 {
        return;
    }

    let was_closed = points.first() == points.last();
    if was_closed {
        points.pop();
    }

    points.reverse();

    if was_closed {
        close_ring(points);
    }
}

fn is_valid_ring(ring: &[Point2]) -> bool {
    ring.len() >= 4 && signed_area(ring).abs() > AREA_EPSILON
}

fn signed_area(points: &[Point2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;
    for index in 0..points.len() {
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        area += current.x * next.y - next.x * current.y;
    }

    area * 0.5
}

fn compare_points(a: Point2, b: Point2) -> std::cmp::Ordering {
    a.x.total_cmp(&b.x).then_with(|| a.y.total_cmp(&b.y))
}

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
}
