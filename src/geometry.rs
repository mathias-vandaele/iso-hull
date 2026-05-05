use crate::types::Point2;

pub(crate) const AREA_EPSILON: f64 = 1.0e-12;

pub(crate) fn circumradius(a: Point2, b: Point2, c: Point2) -> Option<f64> {
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

pub(crate) fn distance(a: Point2, b: Point2) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}

pub(crate) fn close_ring(points: &mut Vec<Point2>) {
    let Some(first) = points.first().copied() else {
        return;
    };

    if points.last().copied() != Some(first) {
        points.push(first);
    }
}

pub(crate) fn ensure_counterclockwise(points: &mut Vec<Point2>) {
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

pub(crate) fn is_valid_ring(ring: &[Point2]) -> bool {
    ring.len() >= 4 && signed_area(ring).abs() > AREA_EPSILON
}

pub(crate) fn signed_area(points: &[Point2]) -> f64 {
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

pub(crate) fn percentile_sorted(values: &[f64], percentile: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }

    let percentile = percentile.clamp(0.0, 1.0);
    let index = (percentile * (values.len() - 1) as f64).ceil() as usize;
    Some(values[index])
}
