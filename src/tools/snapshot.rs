use crate::{
    element::Element,
    math::{edge::Edge, point::Point},
};

pub fn create_element_construct_code_snapshot(element: &Element) -> String {
    let points: Vec<Point> = element
        .shape()
        .edge_iter()
        .filter_map(|v| {
            if let Edge::Line { start_point, .. } = v {
                Some(start_point)
            } else {
                None
            }
        })
        .copied()
        .collect();

    let mut tmp_string = String::new();
    for point in points {
        tmp_string.push_str(&format!("({},{}).into(),", point.x, point.y));
    }

    let angular_velocity = element.meta().angular_velocity();
    let velocity = element.meta().velocity();

    format!(
        "let element = ElementBuilder::new(ConvexPolygon::new(vec![{}]),MetaBuilder::new(10.).angular_velocity({}).velocity(({},{})));",
        tmp_string,
        angular_velocity,
        velocity.x(),
        velocity.y(),
    )
}
