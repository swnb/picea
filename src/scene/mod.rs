pub(crate) mod context;
pub mod errors;
pub(crate) mod hooks;

use std::{collections::BTreeMap, ops::Shl};

use crate::{
    algo::{
        collision::{
            accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
            rough_collision_detection,
        },
        constraint::{solve_join_constraint, ContactManifold, ManifoldsIterMut, Solver},
    },
    element::{self, store::ElementStore, Element},
    manifold::{
        join::{self, JoinManifold},
        Manifold, ManifoldTable,
    },
    math::{point::Point, vector::Vector, FloatNum},
    meta::join::JoinPoint,
    scene::hooks::CallbackHook,
};

use self::context::Context;

pub struct Scene {
    element_store: ElementStore,
    id_dispatcher: IDDispatcher,
    join_point_id_dispatcher: IDDispatcher,
    manifold_table: ManifoldTable,
    join_manifolds: BTreeMap<u32, JoinManifold>,
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
        // TODO use default to gen
        Self {
            element_store: Default::default(),
            id_dispatcher: Default::default(),
            join_point_id_dispatcher: Default::default(),
            manifold_table: Default::default(),
            join_manifolds: Default::default(),
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
    pub fn has_element(&self, element_id: ID) -> bool {
        self.element_store.has_element(element_id)
    }

    #[inline]
    pub fn remove_element(&mut self, element_id: ID) {
        if self.has_element(element_id) {
            self.element_store.remove_element(element_id);
        }
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

        // for _ in 0..4 {

        self.solve_nail_constraints(delta_time);
        // for i in 0..3 {
        self.solve_join_constraints(delta_time);
        // }

        self.integrate_position(delta_time);

        // self.solve_air_friction();
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

    #[inline]
    pub fn get_context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    // remove all elements;
    #[inline]
    pub fn clear(&mut self) {
        self.manifold_table.clear();
        self.element_store.clear();
        self.frame_count = 0;
    }

    pub fn is_element_collide(&self, element_a_id: ID, element_b_id: ID) -> bool {
        let collider_a = self.element_store.get_element_by_id(element_a_id);
        let collider_b = self.element_store.get_element_by_id(element_b_id);
        if let (Some(collider_a), Some(collider_b)) = (collider_a, collider_b) {
            let mut is_collide = false;
            prepare_accurate_collision_detection(
                collider_a,
                collider_b,
                |sub_collider_a, sub_collider_b| {
                    if let Some(contact_constraints) = accurate_collision_detection_for_sub_collider(
                        sub_collider_a,
                        sub_collider_b,
                    ) {
                        is_collide = !contact_constraints.is_empty()
                    }
                },
            );
            is_collide
        } else {
            false
        }
    }

    pub fn pin_element_on_point(&mut self, element_id: ID, point: Point) {
        if let Some(element) = self.get_element_mut(element_id) {
            element.create_nail(point)
        }
    }

    pub fn set_gravity(&mut self, reducer: impl Fn(&Vector) -> Vector) {
        let context = &mut self.context;
        context.default_gravity = reducer(&context.default_gravity);
    }

    pub fn create_join(
        &mut self,
        element_a_id: u32,
        element_a_point: impl Into<Point>,
        element_b_id: u32,
        element_b_point: impl Into<Point>,
        // TODO error
    ) -> Result<(), ()> {
        if element_a_id == element_b_id {
            return Err(());
        }
        let join_point_id = self.join_point_id_dispatcher.gen_id();

        if let Some(element) = self.get_element_mut(element_a_id) {
            element.create_join_point(JoinPoint::new(join_point_id, element_a_point))
        }

        if let Some(element) = self.get_element_mut(element_b_id) {
            element.create_join_point(JoinPoint::new(join_point_id, element_b_point))
        }

        let join_manifold = JoinManifold::new(join_point_id, element_a_id, element_b_id);

        self.join_manifolds.insert(join_point_id, join_manifold);

        Ok(())
    }

    pub fn join_points(&self) -> Vec<(Point, Point)> {
        let query_element_pair = |element_id_pair: (u32, u32)| -> Option<(&Element, &Element)> {
            let element_a =
                self.element_store.get_element_by_id(element_id_pair.0)? as *const Element;

            let element_b =
                self.element_store.get_element_by_id(element_id_pair.1)? as *const Element;

            unsafe { (&*element_a, &*element_b) }.into()
        };

        let result = self
            .join_manifolds
            .values()
            .filter_map(|j| {
                let join_point_id = j.id();
                query_element_pair((j.object_a_id(), j.object_b_id())).and_then(
                    |(element_a, element_b)| {
                        if let Some(join_point_a) = element_a.get_join_point(join_point_id) {
                            let join_point_a = join_point_a as *const JoinPoint;
                            if let Some(join_point_b) = element_b.get_join_point(join_point_id) {
                                let join_point_b = join_point_b as *const JoinPoint;
                                unsafe {
                                    return Some((
                                        *(*join_point_a).point(),
                                        *(*join_point_b).point(),
                                    ));
                                }
                            }
                        }
                        None
                    },
                )
            })
            .collect::<Vec<(Point, Point)>>();
        result
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

    fn integrate_position(&mut self, delta_time: FloatNum) {
        let element_update_callback = &mut self.callback_hook as *mut CallbackHook;

        self.elements_iter_mut().for_each(|element| {
            if let Some((translate, rotation)) = element.integrate_position(delta_time) {
                unsafe {
                    (*element_update_callback).emit(element.id(), translate, rotation);
                }
            }
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
    }
    // TODO
    fn contact_constraint(&mut self) {
        for manifold in self.manifold_table.current_manifolds().iter_mut() {
            let (id_a, id_b) = manifold.collision_element_id_pair();
        }
    }

    fn solve_nail_constraints(&mut self, delta_time: FloatNum) {
        self.elements_iter_mut().for_each(|element| {
            element.solve_nail_constraints(delta_time);
        })
    }

    fn solve_join_constraints(&mut self, delta_time: FloatNum) {
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

        self.join_manifolds
            .values_mut()
            .filter_map(|join_manifold| {
                let join_point_id = join_manifold.id();
                query_element_pair((join_manifold.object_a_id(), join_manifold.object_b_id()))
                    .map(|element_pair| (join_point_id, element_pair))
            })
            .filter_map(|(join_point_id, (element_a, element_b))| {
                if let Some(join_point_a) = element_a.get_join_point_mut(join_point_id) {
                    let join_point_a = join_point_a as *mut JoinPoint;
                    if let Some(join_point_b) = element_b.get_join_point_mut(join_point_id) {
                        let join_point_b = join_point_b as *mut JoinPoint;
                        return Some((element_a, join_point_a, element_b, join_point_b));
                    }
                }
                None
            })
            .for_each(
                |(element_a, join_point_a, element_b, join_point_b)| unsafe {
                    solve_join_constraint(
                        &self.context.constraint_parameters,
                        element_a,
                        (*join_point_a).point(),
                        element_b,
                        (*join_point_b).point(),
                        delta_time,
                    )
                },
            );
    }

    fn solve_air_friction(&mut self) {
        self.elements_iter_mut().for_each(|element| {
            let velocity = element.meta().velocity();
            // TODO OPT abs and powf
            let air_friction = -velocity.normalize() * 0.001 * velocity.abs().powf(2.);

            // TODO replace zero vector
            element
                .meta_mut()
                .apply_impulse(air_friction, (0., 0.).into());
        })
    }

    unsafe fn query_element_pair(
        &self,
        element_a_id: ID,
        element_b_id: ID,
    ) -> Option<(*const Element, *const Element)> {
        if element_a_id == element_b_id {
            return None;
        }

        let element_a = self.element_store.get_element_by_id(element_a_id)? as *const Element;

        let element_b = self.element_store.get_element_by_id(element_b_id)? as *const Element;

        (element_a, element_b).into()
    }

    unsafe fn query_element_pair_mut(
        &mut self,
        element_a_id: ID,
        element_b_id: ID,
    ) -> Option<(*mut Element, *mut Element)> {
        if element_a_id == element_b_id {
            return None;
        }

        let element_a = self.element_store.get_mut_element_by_id(element_a_id)? as *mut Element;

        let element_b = self.element_store.get_mut_element_by_id(element_b_id)? as *mut Element;

        (element_a, element_b).into()
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
