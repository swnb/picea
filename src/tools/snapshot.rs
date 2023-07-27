use crate::{element::Element, math::edge::Edge, shape::utils::check_is_concave};

pub fn create_element_construct_code_snapshot(element: &Element) -> String {
    let points: Vec<_> = element
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

    let is_concave = check_is_concave(&points);

    for point in points {
        tmp_string.push_str(&format!("({:.3},{:.3}).into(),", point.x, point.y));
    }
    let raw_vertexes = tmp_string;

    let mass = element.meta().mass();
    let angular_velocity = element.meta().angular_velocity();
    let velocity = element.meta().velocity();
    let is_fixed = element.meta().is_fixed();

    let element_type = if is_concave {
        "ConcavePolygon"
    } else {
        "ConvexPolygon"
    };

    let mut forces = String::new();

    element.meta().force_group().iter().for_each(|(name, f)| {
        forces.push_str(&format!(
            r#".force("{}", ({:.3}, {:.3}))"#,
            name,
            f.get_vector().x(),
            f.get_vector().y()
        ))
    });

    let is_transparent = element.meta().is_transparent();

    format!(
        r#"let element = ElementBuilder::new(
            {}::new(vec![{}]),
            MetaBuilder::new({:.3})
                .angular_velocity({:.3})
                .velocity(({:.3},{:.3}))
                .is_transparent({})
                .is_fixed({}){});"#,
        element_type,
        raw_vertexes,
        mass,
        angular_velocity,
        velocity.x(),
        velocity.y(),
        is_transparent,
        is_fixed,
        forces,
    )
}
