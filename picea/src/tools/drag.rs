use crate::{
    math::{point::Point, vector::Vector},
    scene::Scene,
    shape::utils::is_point_inside_shape,
};

#[derive(Default)]
pub struct Draggable {
    is_mouse_down: bool,
    mouse_move_point: Option<Point>,
    current_select_element_id: Option<u32>,
}

impl Draggable {
    pub fn on_mouse_down<T>(&mut self, scene: &mut Scene<T>)
    where
        T: Default + Clone,
    {
        self.is_mouse_down = true;
        for element in scene.elements_iter_mut() {
            element.meta_mut().mark_is_transparent(true);
        }
    }

    pub fn on_mouse_move<T>(&mut self, scene: &mut Scene<T>, x: f32, y: f32)
    where
        T: Default + Clone,
    {
        if !self.is_mouse_down {
            return;
        }

        if self.mouse_move_point.is_none() {
            self.on_fist_time_select(scene, x, y);
        } else {
            self.mouse_move_point = Some((x, y).into());
        }

        if let Some(element) = self
            .current_select_element_id
            .and_then(|id| scene.get_element_mut(id))
        {
            let vector_offset: Vector = (element.center_point(), (x, y).into()).into();
            element.translate(&vector_offset);
        }
    }

    fn on_fist_time_select<T>(&mut self, scene: &mut Scene<T>, x: f32, y: f32)
    where
        T: Default + Clone,
    {
        for element in scene.elements_iter() {
            let edge_iter = &mut element.shape().edge_iter();
            if is_point_inside_shape((x, y), edge_iter) {
                self.current_select_element_id = Some(element.id());
                break;
            }
        }
    }

    pub fn on_mouse_up(&mut self) {
        self.is_mouse_down = false;
        self.mouse_move_point = None;
        self.current_select_element_id = None;
    }

    pub fn mouse_point(&self) -> Option<Point> {
        self.mouse_move_point
    }
}
