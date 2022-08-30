use crate::{
    algo::collision::detect_collision,
    element::{store::ElementStore, Element},
    math::point::Point,
    meta::collision::ContactType,
};
use std::rc::Rc;

#[derive(Default)]
pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
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
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }

    #[inline]
    pub fn width_capacity(capacity: usize) -> Self {
        let element_store = ElementStore::with_capacity(capacity);
        Self {
            element_store,
            ..Default::default()
        }
    }

    #[inline]
    pub fn push_element(&mut self, element: impl Into<Element>) {
        let mut element = element.into();
        let element_id = self.id_dispatcher.gen_id();
        element.inject_id(element_id);
        self.element_store.push(element);
    }

    pub fn update_elements_by_duration(
        &mut self,
        delta_time: f32,
        // TODO remove callback
        mut callback: impl FnMut(Vec<Point<f32>>),
    ) {
        self.element_store
            .iter_mut()
            .for_each(|element| element.tick(delta_time));

        detect_collision(&mut self.element_store, |a, b, info| {
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

    #[inline]
    pub fn elements_iter(&self) -> impl Iterator<Item = &Element> {
        self.element_store.iter()
    }

    #[inline]
    pub fn elements_iter_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.element_store.iter_mut()
    }
}
