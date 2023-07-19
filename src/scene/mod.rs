pub(crate) mod context;

use std::{ops::Shl, slice::IterMut};

use crate::{
    algo::{
        collision::{
            accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
            rough_collision_detection,
        },
        constraint::{ContactManifold, ContactPointPairInfo, ManifoldsIterMut, Solver},
    },
    element::{store::ElementStore, Element},
    math::FloatNum,
    meta::collision::{Manifold, ManifoldStore},
};

use self::context::Context;

pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
    manifold_store: ManifoldStore,
    // manifold_store: Vec<Manifold>,
    total_skip_durations: FloatNum,
    context: Context,
    frame_count: u128,
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

impl Default for Scene {
    fn default() -> Self {
        Self::new()
    }
}

impl Scene {
    #[inline]
    pub fn new() -> Self {
        Self {
            element_store: Default::default(),
            id_dispatcher: Default::default(),
            manifold_store: Default::default(),
            // TODO
            total_skip_durations: 0.,
            context: Default::default(),
            frame_count: 0,
        }
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
        self.frame_count += 1;

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

        rough_collision_detection(&mut self.element_store, |element_a, element_b| {
            let should_skip = {
                let meta_a = element_a.meta();
                let meta_b = element_b.meta();

                let is_both_sleeping = meta_a.is_sleeping() && meta_b.is_sleeping();

                is_both_sleeping || meta_a.is_transparent() || meta_b.is_transparent()
            };

            if should_skip {
                return;
            }

            let (collider_a, collider_b) = if element_a.id() > element_b.id() {
                (element_b, element_a)
            } else {
                (element_a, element_b)
            };

            prepare_accurate_collision_detection(
                collider_a,
                collider_b,
                |sub_collider_a, sub_collider_b| {
                    if let Some(contact_point_pairs) = accurate_collision_detection_for_sub_collider(
                        sub_collider_a,
                        sub_collider_b,
                    ) {
                        let a = collider_a;
                        let b = collider_b;
                        // TODO remove mark_collision
                        // a.meta_mut().mark_collision(true);
                        // b.meta_mut().mark_collision(true);

                        let contact_point_pairs = contact_point_pairs
                            .into_iter()
                            .map(|contact_point_pair| (contact_point_pair, a, b).into())
                            .collect();

                        let contact_manifold = Manifold {
                            collision_element_id_pair: (a.id(), b.id()),
                            is_active: true,
                            contact_point_pairs,
                        };

                        self.manifold_store.push(contact_manifold);
                    }
                },
            )
        });

        self.manifold_store.update_all_manifolds_usage();

        self.constraint(delta_time);

        self.elements_iter_mut()
            .for_each(|element| element.integrate_velocity(delta_time));

        // self.manifold_store.update_all_manifolds_usage();
        // self.pre_contact_manifold = std::mem::take(&mut self.contact_manifolds);
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

    fn constraint(&mut self, delta_time: FloatNum) {
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
            type Manifold = Manifold;

            type Iter<'a> = IterMut<'a, Self::Manifold>;

            fn iter_mut(&mut self) -> Self::Iter<'_> {
                <[Manifold]>::iter_mut(self)
            }
        }

        Solver::<'_, '_, _>::new(&self.context, &mut self.manifold_store)
            .constraint(query_element_pair, delta_time);

        // Solver::<'_, '_, _>::new(
        //     &self.context,
        //     &mut self.manifold_store.manifolds_iter_mut_creator(),
        // )
        // .constraint(query_element_pair, delta_time);
    }

    fn frame_count(&self) -> u128 {
        self.frame_count
    }
}

impl<T> Shl<T> for &mut Scene
where
    T: Into<Element>,
{
    type Output = ID;
    fn shl(self, rhs: T) -> Self::Output {
        self.push_element(rhs.into())
    }
}
