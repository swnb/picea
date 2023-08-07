pub(crate) mod context;
pub(crate) mod hooks;

use std::ops::Shl;

use crate::{
    algo::{
        collision::{
            accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
            rough_collision_detection,
        },
        constraint::{ContactManifold, ManifoldsIterMut, Solver},
    },
    element::{store::ElementStore, Element},
    math::{vector::Vector, FloatNum},
    meta::manifold::{Manifold, ManifoldTable},
    scene::hooks::CallbackHook,
};

use self::context::Context;

pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
    manifold_table: ManifoldTable,
    total_skip_durations: FloatNum,
    context: Context,
    frame_count: u128,
    callback_hook: hooks::CallbackHook,
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
            manifold_table: Default::default(),
            // TODO
            total_skip_durations: 0.,
            context: Default::default(),
            frame_count: 0,
            callback_hook: Default::default(),
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
        let mut element: Element = element.into();
        let element_id = self.id_dispatcher.gen_id();
        element.inject_id(element_id);

        self.element_store.push(element);
        element_id
    }

    #[inline]
    pub fn remove_elements(&mut self, element_id: ID) {
        self.element_store.remove_element(element_id);
    }

    #[inline]
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

                let a_v = element.meta().angle_velocity();

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

        self.integrate_velocity(delta_time);

        self.detective_collision();

        self.constraints(delta_time);
    }

    pub fn register_element_position_update_callback<F>(&mut self, callback: F) -> u32
    where
        F: FnMut(ID, Vector, FloatNum) + 'static,
    {
        self.callback_hook.register_callback(callback)
    }

    pub fn unregister_element_position_update_callback(&mut self, callback_id: u32) {
        self.callback_hook.unregister_callback(callback_id);
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

    #[inline]
    pub fn frame_count(&self) -> u128 {
        self.frame_count
    }

    pub fn get_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    // remove all elements;
    pub fn clear(&mut self) {
        self.manifold_table.clear();
        self.element_store.clear();
        self.frame_count = 0;
    }

    fn integrate_velocity(&mut self, delta_time: FloatNum) {
        let gravity = self.context.default_gravity;
        let enable_gravity = self.context.enable_gravity;
        self.elements_iter_mut()
            .filter(|element| !element.meta().is_fixed())
            .filter(|element| !element.meta().is_sleeping())
            .for_each(|element| {
                let force = element.meta().force_group().sum_force();
                let mut a = force * element.meta().inv_mass();
                if enable_gravity {
                    a += gravity;
                }
                element.meta_mut().set_velocity(|pre| pre + a * delta_time);
            });
    }

    fn detective_collision(&mut self) {
        self.manifold_table.flip();

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
                    if let Some(contact_constraints) = accurate_collision_detection_for_sub_collider(
                        sub_collider_a,
                        sub_collider_b,
                    ) {
                        let a = collider_a;
                        let b = collider_b;
                        // TODO remove mark_collision
                        // a.meta_mut().mark_collision(true);
                        // b.meta_mut().mark_collision(true);

                        let contact_constraints = contact_constraints
                            .into_iter()
                            .map(|contact_point_pair| (contact_point_pair, a, b).into())
                            .collect();

                        let contact_manifold = Manifold {
                            collision_element_id_pair: (a.id(), b.id()),
                            reusable: false,
                            contact_constraints,
                        };

                        self.manifold_table.push(contact_manifold);
                    }
                },
            )
        });
    }

    fn constraints(&mut self, delta_time: FloatNum) {
        let query_element_pair =
            &mut |element_id_pair: (u32, u32)| -> Option<(&mut Element, &mut Element)> {
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

        // self.manifold_table.shrink_pre_manifolds();

        self.context.constraint_parameters.skip_friction_constraints = false;

        Solver::<'_, '_, _>::new(&self.context, &mut self.manifold_table.pre_manifolds())
            .constraint(query_element_pair, delta_time);

        self.context.constraint_parameters.skip_friction_constraints = false;

        Solver::<'_, '_, _>::new(&self.context, &mut self.manifold_table.current_manifolds())
            .constraint(query_element_pair, delta_time);

        let element_update_callback = &mut self.callback_hook as *mut CallbackHook;

        self.elements_iter_mut().for_each(|element| {
            if let Some((translate, rotation)) = element.integrate_velocity(delta_time) {
                unsafe {
                    (*element_update_callback).emit(element.id(), translate, rotation);
                }
            }
        });
    }
    // TODO
    fn contact_constraint(&mut self) {
        for manifold in self.manifold_table.current_manifolds().iter_mut() {
            let (id_a, id_b) = manifold.collision_element_id_pair();
        }
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
