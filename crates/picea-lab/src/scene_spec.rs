use picea::{
    math::FloatNum,
    tools::observability::{LabPoint, WorldBounds},
};

pub type TemplateObjectId = u64;

#[derive(Clone, Debug, PartialEq)]
pub struct WorldSpec {
    pub width: FloatNum,
    pub height: FloatNum,
    pub gravity: FloatNum,
    pub editor_clamp: bool,
    pub runtime_boundary: bool,
}

impl WorldSpec {
    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds {
            min: LabPoint { x: 0.0, y: 0.0 },
            max: LabPoint {
                x: self.width.max(1.0),
                y: self.height.max(1.0),
            },
        }
    }

    pub fn default_circle_radius(&self) -> FloatNum {
        (self.width.min(self.height) * 0.05).clamp(1.0, 24.0)
    }

    pub fn default_box_size(&self) -> (FloatNum, FloatNum) {
        let size = (self.width.min(self.height) * 0.12).clamp(2.0, 32.0);
        (size, size)
    }

    pub fn clamp_position(&self, shape: &ObjectShape, position: [FloatNum; 2]) -> [FloatNum; 2] {
        let (half_width, half_height) = shape.half_extents();
        [
            position[0].clamp(half_width, (self.width - half_width).max(half_width)),
            position[1].clamp(half_height, (self.height - half_height).max(half_height)),
        ]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObjectShape {
    Circle { radius: FloatNum },
    Box { width: FloatNum, height: FloatNum },
}

impl ObjectShape {
    pub fn default_circle(world: &WorldSpec) -> Self {
        Self::Circle {
            radius: world.default_circle_radius(),
        }
    }

    pub fn default_box(world: &WorldSpec) -> Self {
        let (width, height) = world.default_box_size();
        Self::Box { width, height }
    }

    pub fn half_extents(&self) -> (FloatNum, FloatNum) {
        match self {
            Self::Circle { radius } => (*radius, *radius),
            Self::Box { width, height } => (*width * 0.5, *height * 0.5),
        }
    }

    pub fn normalize(&mut self) {
        match self {
            Self::Circle { radius } => {
                *radius = radius.abs().max(0.5);
            }
            Self::Box { width, height } => {
                *width = width.abs().max(0.5);
                *height = height.abs().max(0.5);
            }
        }
    }

    pub fn contains_point(&self, center: [FloatNum; 2], point: [FloatNum; 2]) -> bool {
        match self {
            Self::Circle { radius } => {
                let dx = center[0] - point[0];
                let dy = center[1] - point[1];
                (dx * dx) + (dy * dy) <= radius * radius
            }
            Self::Box { width, height } => {
                let half_width = *width * 0.5;
                let half_height = *height * 0.5;
                point[0] >= center[0] - half_width
                    && point[0] <= center[0] + half_width
                    && point[1] >= center[1] - half_height
                    && point[1] <= center[1] + half_height
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectSpec {
    pub id: TemplateObjectId,
    pub position: [FloatNum; 2],
    pub velocity: [FloatNum; 2],
    pub is_fixed: bool,
    pub shape: ObjectShape,
}

impl ObjectSpec {
    pub fn clamped_to(&self, world: &WorldSpec) -> Self {
        let mut object = self.clone();
        object.position = world.clamp_position(&object.shape, object.position);
        object
    }

    pub fn with_position(&self, position: [FloatNum; 2], world: &WorldSpec, clamp: bool) -> Self {
        let mut object = self.clone();
        object.position = if clamp {
            world.clamp_position(&object.shape, position)
        } else {
            position
        };
        object
    }

    pub fn normalized_for_world(&self, world: &WorldSpec, clamp: bool) -> Self {
        let mut object = self.clone();
        object.shape.normalize();
        if clamp {
            object.position = world.clamp_position(&object.shape, object.position);
        }
        object
    }

    pub fn contains_point(&self, point: [FloatNum; 2]) -> bool {
        self.shape.contains_point(self.position, point)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneTemplate {
    pub world: WorldSpec,
    pub objects: Vec<ObjectSpec>,
}

impl SceneTemplate {
    pub fn world_bounds(&self) -> WorldBounds {
        self.world.world_bounds()
    }

    pub fn next_object_id(&self) -> TemplateObjectId {
        self.objects
            .iter()
            .map(|object| object.id)
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[cfg(test)]
mod tests {
    use super::{ObjectShape, ObjectSpec, SceneTemplate, WorldSpec};

    #[test]
    fn scene_template_world_and_object_specs_are_constructible() {
        let template = SceneTemplate {
            world: WorldSpec {
                width: 120.0,
                height: 80.0,
                gravity: 9.8,
                editor_clamp: true,
                runtime_boundary: true,
            },
            objects: vec![
                ObjectSpec {
                    id: 1,
                    position: [20.0, 20.0],
                    velocity: [0.0, 0.0],
                    is_fixed: false,
                    shape: ObjectShape::Circle { radius: 8.0 },
                },
                ObjectSpec {
                    id: 2,
                    position: [70.0, 40.0],
                    velocity: [1.0, 0.0],
                    is_fixed: true,
                    shape: ObjectShape::Box {
                        width: 12.0,
                        height: 10.0,
                    },
                },
            ],
        };

        assert_eq!(template.world.width, 120.0);
        assert_eq!(template.world.height, 80.0);
        assert_eq!(template.objects.len(), 2);
        assert_eq!(template.next_object_id(), 3);
    }

    #[test]
    fn object_spec_normalizes_shape_and_clamps_into_world() {
        let world = WorldSpec {
            width: 20.0,
            height: 12.0,
            gravity: 0.0,
            editor_clamp: true,
            runtime_boundary: false,
        };
        let object = ObjectSpec {
            id: 1,
            position: [25.0, -3.0],
            velocity: [0.0, 0.0],
            is_fixed: false,
            shape: ObjectShape::Box {
                width: -8.0,
                height: 0.1,
            },
        };

        let normalized = object.normalized_for_world(&world, true);

        assert_eq!(
            normalized.shape,
            ObjectShape::Box {
                width: 8.0,
                height: 0.5
            }
        );
        assert!(normalized.position[0] <= 16.0);
        assert!(normalized.position[1] >= 0.25);
    }
}
