use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    geometry::{close_ring, compare_points, ensure_counterclockwise, is_valid_ring, signed_area},
    mesh::{Edge, Triangle},
    types::{Point2, Polygon},
};

pub(crate) fn polygons_from_triangles(triangles: &[Triangle], points: &[Point2]) -> Vec<Polygon> {
    triangle_components(triangles)
        .into_iter()
        .filter_map(|component| polygon_from_component(&component, points))
        .collect()
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
