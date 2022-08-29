use crate::{
    algo::{collision::detect_collision, sort::SortableCollection},
    element::{self, Element},
    math::point::Point,
    meta::collision::ContactType,
};
use std::{cmp::Ordering, collections::BTreeMap, rc::Rc};

#[derive(Default)]
pub struct Scene {
    elements: Vec<Element>,
    id_dispatcher: IDDispatcher,
    sort_result_cache: Option<Vec<usize>>, // for sweep_and_prune sort result cache
    index_map: BTreeMap<ID, usize>,
}

impl SortableCollection for Scene {
    type Item = Element;
    fn init(&mut self) {
        if self.sort_result_cache.is_none() {
            let len = self.elements.len();
            let init_element_index_array = (0..len).collect();
            self.sort_result_cache = Some(init_element_index_array);
        }
    }

    fn len(&self) -> usize {
        self.elements.len()
    }

    fn get(&self, index: usize) -> &Element {
        let index = self.sort_result_cache.as_ref().map(|v| v[index]).unwrap();
        &self.elements[index]
    }

    fn swap(&mut self, i: usize, j: usize) {
        if let Some(element_index_array) = self.sort_result_cache.as_mut() {
            element_index_array.swap(i, j);
        }
    }
}

type ID = u32;

/**
 * uuid generator
 */
struct IDDispatcher {
    current_id: ID,
}

impl Default for IDDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl IDDispatcher {
    const fn new() -> Self {
        Self { current_id: 0 }
    }

    fn gen_id(&mut self) -> u32 {
        self.current_id = self.current_id.checked_add(1).expect("create too much id");
        self.current_id
    }
}

impl Scene {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn width_capacity(capacity: usize) -> Self {
        let elements = Vec::with_capacity(capacity);
        Self {
            elements,
            ..Default::default()
        }
    }

    pub fn push_element(&mut self, element: impl Into<Element>) {
        let mut element = element.into();
        let element_id = self.id_dispatcher.gen_id();
        element.inject_id(element_id);
        let index = self.elements.len();
        self.elements.push(element);
        self.index_map.insert(element_id, index);
    }

    pub fn update_elements_by_duration(
        &mut self,
        delta_time: f32,
        // TODO remove callback
        mut callback: impl FnMut(Vec<Point<f32>>),
    ) {
        self.elements
            .iter_mut()
            .for_each(|element| element.tick(delta_time));

        detect_collision(&mut self.elements, |a, b, info| {
            a.meta_mut().mark_collision(true);
            b.meta_mut().mark_collision(true);

            // TODO

            let info = Rc::new(info);

            a.meta_mut().set_collision_infos(info.clone());
            b.meta_mut().set_collision_infos(info.clone());

            let contact_a = info.contact_a();

            let contact_b = info.contact_b();
            dbg!(contact_a);
            dbg!(contact_b);

            let mut l = match contact_a {
                ContactType::Point(p) => vec![*p],
                ContactType::Edge([p, p2]) => vec![*p, *p2],
            };

            let l1 = match contact_b {
                ContactType::Point(p) => vec![*p],
                ContactType::Edge([p, p2]) => vec![*p, *p2],
            };

            let normal = info.normal() * 10.;

            l.extend(l1);

            l.push((normal.x(), normal.y()).into());

            callback(l);
            // a.force_group_mut()
            //     .add_force(Force::new("pop", -normal * 10.));
            // b.force_group_mut()
            //     .add_force(Force::new("pop", normal * 10.));
        });
    }

    pub fn elements_iter(&self) -> impl Iterator<Item = &Element> {
        self.elements.iter()
    }

    pub fn elements_iter_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.elements.iter_mut()
    }
}
