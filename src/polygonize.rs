use std::time::Instant;

use log::trace;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    geometry::{close_ring, ensure_counterclockwise, is_valid_ring, signed_area, AREA_EPSILON},
    mesh::Triangle,
    types::{Point2, Polygon},
};

pub(crate) fn polygons_from_triangles(triangles: &[Triangle], points: &[Point2]) -> Vec<Polygon> {
    let total_started = Instant::now();

    let started = Instant::now();
    let boundary_edges = boundary_edge_keys(triangles);
    trace!(
        target: "isohull::polygonize",
        "boundary_edges input_triangles={} boundary_edges={} elapsed_ms={:.3}",
        triangles.len(),
        boundary_edges.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let rings = extract_rings(&boundary_edges, points);
    trace!(
        target: "isohull::polygonize",
        "extract_rings boundary_edges={} rings={} elapsed_ms={:.3}",
        boundary_edges.len(),
        rings.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    let started = Instant::now();
    let polygons = rings_to_polygons(rings);
    trace!(
        target: "isohull::polygonize",
        "rings_to_polygons polygons={} elapsed_ms={:.3}",
        polygons.len(),
        started.elapsed().as_secs_f64() * 1000.0
    );

    trace!(
        target: "isohull::polygonize",
        "polygons_from_triangles.total input_triangles={} polygons={} elapsed_ms={:.3}",
        triangles.len(),
        polygons.len(),
        total_started.elapsed().as_secs_f64() * 1000.0
    );

    polygons
}

fn boundary_edge_keys(triangles: &[Triangle]) -> Vec<u64> {
    let mut counts = FxHashMap::default();
    counts.reserve(triangles.len().saturating_mul(3));

    for triangle in triangles {
        count_edge(&mut counts, triangle.a, triangle.b);
        count_edge(&mut counts, triangle.b, triangle.c);
        count_edge(&mut counts, triangle.c, triangle.a);
    }

    let mut boundary_edges = Vec::with_capacity(counts.len());
    for (key, count) in counts {
        if count == 1 {
            boundary_edges.push(key);
        }
    }

    boundary_edges.sort_unstable();
    boundary_edges
}

#[inline(always)]
fn count_edge(counts: &mut FxHashMap<u64, u8>, a: usize, b: usize) {
    let key = edge_key(a, b);
    let count = counts.entry(key).or_insert(0);
    *count = count.saturating_add(1);
}

fn extract_rings(edges: &[u64], points: &[Point2]) -> Vec<Vec<Point2>> {
    if edges.is_empty() {
        return Vec::new();
    }

    let mut adjacency = boundary_adjacency(edges, points.len());

    if is_simple_boundary_graph(&adjacency) {
        extract_simple_rings(edges, &adjacency, points)
    } else {
        sort_neighbors_by_angle(&mut adjacency, points);
        extract_planar_face_rings(edges, &adjacency, points)
    }
}

fn boundary_adjacency(edges: &[u64], point_count: usize) -> Vec<Vec<usize>> {
    let mut degrees = vec![0usize; point_count];

    for &key in edges {
        let u = edge_u(key);
        let v = edge_v(key);
        degrees[u] += 1;
        degrees[v] += 1;
    }

    let mut adjacency = Vec::with_capacity(point_count);
    for degree in degrees {
        adjacency.push(Vec::with_capacity(degree));
    }

    for &key in edges {
        let u = edge_u(key);
        let v = edge_v(key);
        adjacency[u].push(v);
        adjacency[v].push(u);
    }

    for neighbors in &mut adjacency {
        if neighbors.len() > 1 {
            neighbors.sort_unstable();
            neighbors.dedup();
        }
    }

    adjacency
}

fn is_simple_boundary_graph(adjacency: &[Vec<usize>]) -> bool {
    for neighbors in adjacency {
        if !neighbors.is_empty() && neighbors.len() != 2 {
            return false;
        }
    }

    true
}

fn extract_simple_rings(
    edges: &[u64],
    adjacency: &[Vec<usize>],
    points: &[Point2],
) -> Vec<Vec<Point2>> {
    let mut visited = FxHashSet::default();
    visited.reserve(edges.len());

    let mut rings = Vec::new();
    for &key in edges {
        if visited.contains(&key) {
            continue;
        }

        if let Some(ring) = walk_simple_cycle(key, adjacency, points, edges.len(), &mut visited) {
            rings.push(ring);
        }
    }

    rings
}

fn walk_simple_cycle(
    start_key: u64,
    adjacency: &[Vec<usize>],
    points: &[Point2],
    max_edges: usize,
    visited: &mut FxHashSet<u64>,
) -> Option<Vec<Point2>> {
    let start = edge_u(start_key);
    let mut previous = start;
    let mut current = edge_v(start_key);
    let mut ring_indices = Vec::with_capacity(64);
    ring_indices.push(start);

    for _ in 0..=max_edges {
        let key = edge_key(previous, current);
        if !visited.insert(key) {
            return None;
        }

        ring_indices.push(current);
        if current == start {
            break;
        }

        let neighbors = &adjacency[current];
        if neighbors.len() != 2 {
            return None;
        }

        let next = if neighbors[0] == previous {
            neighbors[1]
        } else if neighbors[1] == previous {
            neighbors[0]
        } else {
            return None;
        };

        previous = current;
        current = next;
    }

    if ring_indices.last().copied() != Some(start) {
        return None;
    }

    if ring_indices.len() < 4 {
        return None;
    }

    let mut ring = Vec::with_capacity(ring_indices.len());
    for index in ring_indices {
        ring.push(points[index]);
    }

    Some(ring)
}

fn sort_neighbors_by_angle(adjacency: &mut [Vec<usize>], points: &[Point2]) {
    for vertex in 0..adjacency.len() {
        if adjacency[vertex].len() <= 1 {
            continue;
        }

        adjacency[vertex].sort_unstable_by(|a, b| {
            angle(points[vertex], points[*a]).total_cmp(&angle(points[vertex], points[*b]))
        });
        adjacency[vertex].dedup();
    }
}

fn extract_planar_face_rings(
    edges: &[u64],
    adjacency: &[Vec<usize>],
    points: &[Point2],
) -> Vec<Vec<Point2>> {
    let mut visited = FxHashSet::default();
    visited.reserve(edges.len().saturating_mul(2));

    let mut rings = Vec::new();
    for &key in edges {
        let u = edge_u(key);
        let v = edge_v(key);

        if let Some(ring) = walk_planar_face(u, v, adjacency, points, edges.len(), &mut visited) {
            rings.push(ring);
        }
        if let Some(ring) = walk_planar_face(v, u, adjacency, points, edges.len(), &mut visited) {
            rings.push(ring);
        }
    }

    rings
}

fn walk_planar_face(
    start_from: usize,
    start_to: usize,
    adjacency: &[Vec<usize>],
    points: &[Point2],
    max_edges: usize,
    visited: &mut FxHashSet<u64>,
) -> Option<Vec<Point2>> {
    if visited.contains(&directed_edge_key(start_from, start_to)) {
        return None;
    }

    let mut from = start_from;
    let mut to = start_to;
    let mut ring_indices = Vec::with_capacity(64);

    for _ in 0..=max_edges {
        if !visited.insert(directed_edge_key(from, to)) {
            return None;
        }

        ring_indices.push(from);

        let neighbors = &adjacency[to];
        let incoming = neighbors.iter().position(|&neighbor| neighbor == from)?;
        let next = neighbors[(incoming + neighbors.len() - 1) % neighbors.len()];

        from = to;
        to = next;

        if from == start_from && to == start_to {
            break;
        }
    }

    if from != start_from || to != start_to {
        return None;
    }

    ring_indices.push(start_from);

    let mut ring = Vec::with_capacity(ring_indices.len());
    for index in ring_indices {
        ring.push(points[index]);
    }

    if signed_area(&ring) > AREA_EPSILON {
        Some(ring)
    } else {
        None
    }
}

fn rings_to_polygons(rings: Vec<Vec<Point2>>) -> Vec<Polygon> {
    let mut rings = valid_ring_candidates(rings);
    rings.sort_by(|a, b| b.abs_area.total_cmp(&a.abs_area));

    let mut keep = Vec::with_capacity(rings.len());
    for index in 0..rings.len() {
        keep.push(containment_depth(index, &rings) % 2 == 0);
    }

    let mut polygons = Vec::with_capacity(rings.len());
    for (candidate, keep) in rings.into_iter().zip(keep) {
        if keep {
            polygons.push(Polygon {
                outer: candidate.points,
            });
        }
    }

    polygons
}

fn valid_ring_candidates(rings: Vec<Vec<Point2>>) -> Vec<RingCandidate> {
    let mut candidates = Vec::with_capacity(rings.len());

    for mut points in rings {
        close_ring(&mut points);

        if !is_valid_ring(&points) {
            continue;
        }

        let abs_area = signed_area(&points).abs();
        ensure_counterclockwise(&mut points);

        candidates.push(RingCandidate { points, abs_area });
    }

    candidates
}

fn containment_depth(index: usize, rings: &[RingCandidate]) -> usize {
    let point = rings[index].points[0];
    let mut depth = 0;

    for candidate in &rings[..index] {
        if point_in_ring(point, &candidate.points) {
            depth += 1;
        }
    }

    depth
}

fn point_in_ring(point: Point2, ring: &[Point2]) -> bool {
    let mut inside = false;

    for index in 0..ring.len() - 1 {
        let a = ring[index];
        let b = ring[index + 1];
        let crosses = (a.y > point.y) != (b.y > point.y);

        if crosses {
            let x = (b.x - a.x) * (point.y - a.y) / (b.y - a.y) + a.x;
            if point.x < x {
                inside = !inside;
            }
        }
    }

    inside
}

#[derive(Debug)]
struct RingCandidate {
    points: Vec<Point2>,
    abs_area: f64,
}

#[inline(always)]
fn edge_key(a: usize, b: usize) -> u64 {
    debug_assert!(a <= u32::MAX as usize);
    debug_assert!(b <= u32::MAX as usize);

    let lo = a.min(b) as u64;
    let hi = a.max(b) as u64;
    (lo << 32) | hi
}

#[inline(always)]
fn directed_edge_key(from: usize, to: usize) -> u64 {
    debug_assert!(from <= u32::MAX as usize);
    debug_assert!(to <= u32::MAX as usize);

    ((from as u64) << 32) | to as u64
}

#[inline(always)]
fn edge_u(key: u64) -> usize {
    (key >> 32) as usize
}

#[inline(always)]
fn edge_v(key: u64) -> usize {
    (key & 0xffff_ffff) as usize
}

#[inline(always)]
fn angle(origin: Point2, point: Point2) -> f64 {
    (point.y - origin.y).atan2(point.x - origin.x)
}
