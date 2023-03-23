use crate::{
    algo::{collision::detect_collision, constraint::constraint},
    element::{store::ElementStore, Element},
    math::CommonNum,
    meta::collision::CollisionInfo,
};

#[derive(Default)]
pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
    contact_manifolds: Vec<CollisionInfo>,
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
    pub fn push_element(&mut self, element: impl Into<Element>) -> u32 {
        let mut element = element.into();
        let element_id = self.id_dispatcher.gen_id();
        element.inject_id(element_id);
        self.element_store.push(element);
        element_id
    }

    pub fn update_elements_by_duration(&mut self, delta_time: f32) {
        // self.element_store
        //     .iter_mut()
        //     .for_each(|element| element.tick(delta_time));

        self.element_store.iter_mut().for_each(|element| {
            let force = element.meta().force_group().sum_force();
            let a = force * element.meta().inv_mass();
            element.meta_mut().set_velocity(|pre| pre + a * delta_time);
        });

        self.contact_manifolds.clear();

        detect_collision(&mut self.element_store, |a, b, contact_point_pairs| {
            // TODO remove mark_collision
            // a.meta_mut().mark_collision(true);
            // b.meta_mut().mark_collision(true);

            let collision_infos =
                contact_point_pairs
                    .into_iter()
                    .map(|contact_point_pair| CollisionInfo {
                        collision_element_id_pair: (a.id(), b.id()),
                        contact_point_pair,
                        mass_effective: None,
                    });

            self.contact_manifolds.extend(collision_infos);
        });

        for _ in 0..10 {
            self.constraint(delta_time, false);
        }

        self.constraint(delta_time, true);

        self.element_store
            .iter_mut()
            .for_each(|element| element.integrate_velocity(delta_time))
    }

    #[inline]
    pub fn elements_iter(&self) -> impl Iterator<Item = &Element> {
        self.element_store.iter()
    }

    #[inline]
    pub fn elements_iter_mut(&mut self) -> impl Iterator<Item = &mut Element> {
        self.element_store.iter_mut()
    }

    #[inline]
    pub fn get_element(&self, id: ID) -> Option<&Element> {
        self.element_store.get_element_by_id(id)
    }

    #[inline]
    pub fn get_element_mut(&mut self, id: ID) -> Option<&mut Element> {
        self.element_store.get_mut_element_by_id(id)
    }

    pub(crate) fn query_element_pair(
        &mut self,
        element_id_pair: (u32, u32),
    ) -> Option<(&mut Element, &mut Element)> {
        let element_a = self
            .element_store
            .get_mut_element_by_id(element_id_pair.0)? as *mut Element;

        let element_b = self
            .element_store
            .get_mut_element_by_id(element_id_pair.1)? as *mut Element;

        unsafe { (&mut *element_a, &mut *element_b) }.into()
    }

    fn constraint(&mut self, delta_time: CommonNum, should_use_bias: bool) {
        let query_element_pair =
            |element_id_pair: (u32, u32)| -> Option<(&mut Element, &mut Element)> {
                let element_a = self
                    .element_store
                    .get_mut_element_by_id(element_id_pair.0)?
                    as *mut Element;

                let element_b = self
                    .element_store
                    .get_mut_element_by_id(element_id_pair.1)?
                    as *mut Element;

                unsafe { (&mut *element_a, &mut *element_b) }.into()
            };

        constraint(
            self.contact_manifolds.iter_mut(),
            query_element_pair,
            delta_time,
            should_use_bias,
        );
    }
}
