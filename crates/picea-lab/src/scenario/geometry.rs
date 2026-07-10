use crate::{LabError, LabResult};

/// The scene fixture owns stable authoring errors for the validated fixture
/// shape fields below, so these failures do not bounce back from recipe paths.
pub(super) fn validate_convex_vertices(path: &str, vertices: &[[f32; 2]]) -> LabResult<()> {
    let has_enough_vertices = vertices.len() >= 3 && distinct_vertex_count(vertices) >= 3;
    let vertices_are_finite = polygon_vertices_are_finite(vertices);
    let has_no_zero_length_edges = polygon_has_no_zero_length_edges(vertices);
    let has_area = polygon_twice_area(vertices).abs() > f32::EPSILON;

    if !has_enough_vertices || !vertices_are_finite || !has_no_zero_length_edges || !has_area {
        Err(LabError::World(format!(
            "{path}: convex_polygon requires at least 3 non-degenerate vertices"
        )))
    } else if polygon_is_convex(vertices) {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: convex_polygon requires convex vertices"
        )))
    }
}

pub(super) fn decompose_concave_polygon(
    path: &str,
    vertices: &[[f32; 2]],
) -> LabResult<Vec<Vec<[f32; 2]>>> {
    validate_concave_polygon_for_decomposition(path, vertices)?;

    let mut oriented = vertices.to_vec();
    if polygon_twice_area(&oriented) < 0.0 {
        oriented.reverse();
    }

    let mut indices = (0..oriented.len()).collect::<Vec<_>>();
    let mut pieces = Vec::with_capacity(oriented.len().saturating_sub(2));
    while indices.len() > 3 {
        let Some(ear_position) = find_next_ear(&oriented, &indices) else {
            return Err(LabError::World(format!(
                "{path}: concave_polygon could not be decomposed deterministically"
            )));
        };
        let previous = indices[(ear_position + indices.len() - 1) % indices.len()];
        let current = indices[ear_position];
        let next = indices[(ear_position + 1) % indices.len()];
        pieces.push(vec![oriented[previous], oriented[current], oriented[next]]);
        indices.remove(ear_position);
    }
    pieces.push(indices.iter().map(|index| oriented[*index]).collect());
    Ok(pieces)
}

fn validate_concave_polygon_for_decomposition(path: &str, vertices: &[[f32; 2]]) -> LabResult<()> {
    let has_enough_vertices = vertices.len() >= 4 && distinct_vertex_count(vertices) >= 4;
    let vertices_are_finite = polygon_vertices_are_finite(vertices);
    let has_no_zero_length_edges = polygon_has_no_zero_length_edges(vertices);
    let has_area = polygon_twice_area(vertices).abs() > f32::EPSILON;

    if !has_enough_vertices || !vertices_are_finite || !has_no_zero_length_edges || !has_area {
        return Err(LabError::World(format!(
            "{path}: concave_polygon requires at least 4 non-degenerate vertices"
        )));
    }
    if polygon_has_self_intersections(vertices) {
        return Err(LabError::World(format!(
            "{path}: concave_polygon must be simple and non-self-intersecting"
        )));
    }
    if polygon_is_convex(vertices) {
        return Err(LabError::World(format!(
            "{path}: concave_polygon requires a concave vertex; use convex_polygon for convex loops"
        )));
    }
    Ok(())
}

fn find_next_ear(vertices: &[[f32; 2]], indices: &[usize]) -> Option<usize> {
    for position in 0..indices.len() {
        let previous = indices[(position + indices.len() - 1) % indices.len()];
        let current = indices[position];
        let next = indices[(position + 1) % indices.len()];
        if polygon_turn_cross(vertices[previous], vertices[current], vertices[next]) <= f32::EPSILON
        {
            continue;
        }
        if indices
            .iter()
            .copied()
            .filter(|index| *index != previous && *index != current && *index != next)
            .any(|index| {
                point_in_triangle(
                    vertices[index],
                    vertices[previous],
                    vertices[current],
                    vertices[next],
                )
            })
        {
            continue;
        }
        return Some(position);
    }
    None
}

fn point_in_triangle(point: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let ab = polygon_turn_cross(a, b, point);
    let bc = polygon_turn_cross(b, c, point);
    let ca = polygon_turn_cross(c, a, point);
    ab >= -f32::EPSILON && bc >= -f32::EPSILON && ca >= -f32::EPSILON
}

pub(super) fn validate_circle_radius(path: &str, radius: f32) -> LabResult<()> {
    if radius.is_finite() && radius > 0.0 {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: circle radius must be finite and > 0"
        )))
    }
}

pub(super) fn validate_rect_size(path: &str, width: f32, height: f32) -> LabResult<()> {
    validate_positive_finite_scalar(&format!("{path}.width"), width, "rect width")?;
    validate_positive_finite_scalar(&format!("{path}.height"), height, "rect height")
}

pub(super) fn validate_local_pose(path: &str, [x, y, angle]: [f32; 3]) -> LabResult<()> {
    validate_finite_scalar(&format!("{path}.x"), x, "local_pose.x")?;
    validate_finite_scalar(&format!("{path}.y"), y, "local_pose.y")?;
    validate_finite_scalar(&format!("{path}.angle"), angle, "local_pose.angle")
}

fn validate_positive_finite_scalar(path: &str, value: f32, label: &str) -> LabResult<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(LabError::World(format!(
            "{path}: {label} must be finite and > 0"
        )))
    }
}

fn validate_finite_scalar(path: &str, value: f32, label: &str) -> LabResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(LabError::World(format!("{path}: {label} must be finite")))
    }
}

fn distinct_vertex_count(vertices: &[[f32; 2]]) -> usize {
    let mut distinct = Vec::with_capacity(vertices.len());
    for vertex in vertices {
        if !distinct.iter().any(|existing| existing == vertex) {
            distinct.push(*vertex);
        }
    }
    distinct.len()
}

fn polygon_vertices_are_finite(vertices: &[[f32; 2]]) -> bool {
    vertices.iter().all(|[x, y]| x.is_finite() && y.is_finite())
}

fn polygon_has_no_zero_length_edges(vertices: &[[f32; 2]]) -> bool {
    if vertices.len() < 2 {
        return true;
    }

    for index in 0..vertices.len() {
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let edge_x = next[0] - current[0];
        let edge_y = next[1] - current[1];
        if edge_x.abs() <= f32::EPSILON && edge_y.abs() <= f32::EPSILON {
            return false;
        }
    }

    true
}

fn polygon_has_self_intersections(vertices: &[[f32; 2]]) -> bool {
    for first in 0..vertices.len() {
        let first_next = (first + 1) % vertices.len();
        for second in (first + 1)..vertices.len() {
            let second_next = (second + 1) % vertices.len();
            if first == second_next || first_next == second {
                continue;
            }
            if segments_intersect(
                vertices[first],
                vertices[first_next],
                vertices[second],
                vertices[second_next],
            ) {
                return true;
            }
        }
    }
    false
}

fn segments_intersect(a: [f32; 2], b: [f32; 2], c: [f32; 2], d: [f32; 2]) -> bool {
    let d1 = point_line_orientation(a, b, c);
    let d2 = point_line_orientation(a, b, d);
    let d3 = point_line_orientation(c, d, a);
    let d4 = point_line_orientation(c, d, b);

    if d1.abs() <= f32::EPSILON && point_on_segment(c, a, b) {
        return true;
    }
    if d2.abs() <= f32::EPSILON && point_on_segment(d, a, b) {
        return true;
    }
    if d3.abs() <= f32::EPSILON && point_on_segment(a, c, d) {
        return true;
    }
    if d4.abs() <= f32::EPSILON && point_on_segment(b, c, d) {
        return true;
    }

    d1.signum() != d2.signum() && d3.signum() != d4.signum()
}

fn point_line_orientation(a: [f32; 2], b: [f32; 2], point: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (point[1] - a[1]) - (b[1] - a[1]) * (point[0] - a[0])
}

fn point_on_segment(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> bool {
    point[0] >= a[0].min(b[0]) - f32::EPSILON
        && point[0] <= a[0].max(b[0]) + f32::EPSILON
        && point[1] >= a[1].min(b[1]) - f32::EPSILON
        && point[1] <= a[1].max(b[1]) + f32::EPSILON
}

fn polygon_twice_area(vertices: &[[f32; 2]]) -> f32 {
    if vertices.len() < 3 {
        return 0.0;
    }

    let mut twice_area = 0.0;
    for index in 0..vertices.len() {
        let [x1, y1] = vertices[index];
        let [x2, y2] = vertices[(index + 1) % vertices.len()];
        twice_area += x1 * y2 - x2 * y1;
    }
    twice_area
}

/// "Convex" here means the authored loop turns consistently around the shape.
/// Collinear edges are tolerated so authors can keep explicit seam vertices.
fn polygon_is_convex(vertices: &[[f32; 2]]) -> bool {
    let mut winding_sign = 0.0_f32;

    for index in 0..vertices.len() {
        let previous = vertices[(index + vertices.len() - 1) % vertices.len()];
        let current = vertices[index];
        let next = vertices[(index + 1) % vertices.len()];
        let turn = polygon_turn_cross(previous, current, next);
        if turn.abs() <= f32::EPSILON {
            continue;
        }

        if winding_sign == 0.0 {
            winding_sign = turn.signum();
            continue;
        }

        if turn.signum() != winding_sign {
            return false;
        }
    }

    true
}

fn polygon_turn_cross(previous: [f32; 2], current: [f32; 2], next: [f32; 2]) -> f32 {
    let incoming_x = current[0] - previous[0];
    let incoming_y = current[1] - previous[1];
    let outgoing_x = next[0] - current[0];
    let outgoing_y = next[1] - current[1];
    incoming_x * outgoing_y - incoming_y * outgoing_x
}
