pub(crate) mod context;

use std::slice::IterMut;

use crate::{
    algo::{
        collision::detect_collision,
        constraint::{ManifoldsIterMut, Solver},
    },
    element::{store::ElementStore, Element},
    math::FloatNum,
    meta::collision::Manifold,
};

use self::context::Context;

#[derive(Default)]
pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
    contact_manifolds: Vec<Manifold>,
    pre_contact_manifold: Vec<Manifold>,
    total_skip_durations: FloatNum,
    context: Context,
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

enum SceneManifoldsType {
    PreviousManifolds,
    CurrentManifolds,
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

    pub fn element_size(&self) -> usize {
        self.element_store.size()
    }

    pub fn update_elements_by_duration(&mut self, delta_time: f32) {
        let Context {
            max_enter_sleep_frame,
            max_enter_sleep_motion,
            ..
        } = self.context;

        if self.context.enable_sleep_mode {
            self.elements_iter_mut().for_each(|element| {
                let v = element.meta().velocity();

                let a_v = element.meta().angular_velocity();

                // TODO better performance for abs
                let motion = v.abs().powf(2.) + a_v.powf(2.);

                if motion < max_enter_sleep_motion {
                    element.meta_mut().mark_motionless();
                    if element.meta().motionless_frame_counter() > max_enter_sleep_frame {
                        element.meta_mut().reset_motionless_frame_counter();
                        element.meta_mut().mark_is_sleeping(true);
                    }
                } else {
                    element.meta_mut().mark_is_sleeping(false);
                }
            });
        }

        // TODO 120 fps
        // max frame rate is 60
        const MIN_DELTA_TIME: FloatNum = 1. / 61.;
        // if self.total_skip_durations + delta_time < MIN_DELTA_TIME {
        //     // skip this frame
        //     self.total_skip_durations += delta_time;
        //     return;
        // }

        // let delta_time = self.total_skip_durations + delta_time;

        // self.total_skip_durations = 0.;

        // TODO use dynamic delta_time

        let delta_time: FloatNum = 1. / 61.;

        // self.element_store
        //     .iter_mut()
        //     .for_each(|element| element.tick(delta_time));

        self.elements_iter_mut()
            .filter(|element| !element.meta().is_fixed())
            .filter(|element| !element.meta().is_sleeping())
            .for_each(|element| {
                let force = element.meta().force_group().sum_force();
                let a = force * element.meta().inv_mass();
                element.meta_mut().set_velocity(|pre| pre + a * delta_time);
            });

        detect_collision(
            &mut self.element_store,
            |a, b, contact_point_pairs| {
                // TODO remove mark_collision
                // a.meta_mut().mark_collision(true);
                // b.meta_mut().mark_collision(true);

                let contact_point_pairs = contact_point_pairs
                    .into_iter()
                    .map(|contact_point_pair| (contact_point_pair, a, b).into())
                    .collect();

                let contact_manifold = Manifold {
                    collision_element_id_pair: (a.id(), b.id()),
                    contact_point_pairs,
                };

                self.contact_manifolds.push(contact_manifold);
            },
            |element_a, element_b| {
                let meta_a = element_a.meta();
                let meta_b = element_b.meta();
                meta_a.is_transparent()
                    || meta_b.is_transparent()
                    || (meta_a.is_sleeping() && meta_b.is_sleeping())
            },
        );

        use SceneManifoldsType::*;

        self.constraint(PreviousManifolds, delta_time);

        self.constraint(CurrentManifolds, delta_time);

        self.elements_iter_mut()
            .for_each(|element| element.integrate_velocity(delta_time));

        self.pre_contact_manifold = std::mem::take(&mut self.contact_manifolds);
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

    fn constraint(&mut self, manifolds_type: SceneManifoldsType, delta_time: FloatNum) {
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

        impl ManifoldsIterMut for [Manifold] {
            type Item<'z> = IterMut<'z, Manifold> where Self:'z;

            fn iter_mut(&mut self) -> Self::Item<'_> {
                <[Manifold]>::iter_mut(self)
            }
        }

        use SceneManifoldsType::*;
        match manifolds_type {
            CurrentManifolds => {
                Solver::<'_, '_, [Manifold]>::new(&self.context, &mut self.contact_manifolds)
                    .constraint(query_element_pair, delta_time);
            }
            PreviousManifolds => {
                Solver::<'_, '_, [Manifold]>::new(&self.context, &mut self.pre_contact_manifold)
                    .constraint(query_element_pair, delta_time);
            }
        }
    }
}
