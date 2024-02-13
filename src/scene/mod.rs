pub(crate) mod context;
pub mod errors;
pub(crate) mod hooks;

use std::{collections::BTreeMap, ops::Shl};

use crate::{
    collision::{
        accurate_collision_detection_for_sub_collider, prepare_accurate_collision_detection,
    },
    constraints::{
        contact::ContactConstraint, join::JoinConstraint, point::PointConstraint,
        JoinConstraintConfig,
    },
    element::{store::ElementStore, Element},
    math::{point::Point, vector::Vector, FloatNum},
    scene::hooks::CallbackHook,
};

use self::context::Context;

#[derive(Default)]
pub struct Scene<Data = ()>
where
    Data: Clone + Default,
{
    pub(crate) element_store: ElementStore<Data>,
    id_dispatcher: IDDispatcher,
    total_duration: FloatNum,
    total_skip_durations: FloatNum,
    pub(crate) context: Context,
    frame_count: u128,
    callback_hook: hooks::CallbackHook,
    constraints_id_dispatcher: IDDispatcher,
    pub(crate) previous_contact_constraints: Vec<ContactConstraint<Element<Data>>>,
    pub(crate) contact_constraints: Vec<ContactConstraint<Element<Data>>>,
    point_constraints: BTreeMap<u32, PointConstraint<Element<Data>>>,
    join_constraints: BTreeMap<u32, JoinConstraint<Element<Data>>>,
    pub data: Data,
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

    fn reset(&mut self) {
        self.current_id = 0;
    }
}

impl<T: Clone + Default> Scene<T> {
    #[inline]
    pub fn new() -> Self {
        Scene {
            ..Default::default()
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
    pub fn push_element(&mut self, element: impl Into<Element<T>>) -> u32 {
        let mut element: Element<T> = element.into();

        let element_id = self.id_dispatcher.gen_id();
        element.inject_id(element_id);

        let center_point = element.shape().center_point();

        element.meta_mut().set_position(|_| center_point);

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

    pub fn total_duration(&self) -> FloatNum {
        self.total_duration
    }

    pub fn update_elements_by_duration(&mut self, delta_time: f32) {
        self.frame_count += 1;

        let Context {
            max_enter_sleep_frame,
            max_enter_sleep_motion,
            ..
        } = self.context;

        self.elements_iter_mut().for_each(|element| {
            element.meta_mut().sync_position().sync_angle();
        });

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

        self.total_duration += delta_time;

        self.integrate_velocity(delta_time);

        self.previous_contact_constraints = std::mem::take(&mut self.contact_constraints);

        self.element_store
            .detective_collision(|a, b, mut contact_pairs| {
                if a.id() == 2 {
                    dbg!(&contact_pairs);
                } else if b.id() == 2 {
                    dbg!(&contact_pairs);
                }

                contact_pairs
                    .into_iter()
                    // .rev()
                    // TODO opt
                    .for_each(|pair| {
                        if let Some(previous_contact) =
                            self.contact_constraints.iter_mut().find(|constraint| {
                                // // 上一次用过的
                                // !constraint.is_active()
                                //     && constraint.obj_id_pair() == (a.id(), b.id())
                                //     && constraint.contact_point_pair() == &pair
                                false
                            })
                        {
                            previous_contact.update_contact_point_pair(pair);
                            previous_contact.set_active(true);
                        } else {
                            self.contact_constraints.push(ContactConstraint::new(
                                a.id(),
                                b.id(),
                                pair,
                            ));
                        }
                    });
            });

        unsafe {
            self.reset_constraints_params(delta_time);
        }

        const MAX_CONSTRAINTS_TIMES: u8 = 20;

        dbg!(self
            .get_element(2)
            .map(|element| { element.meta().angle() }));

        dbg!(self
            .get_element(3)
            .map(|element| { element.meta().angle() }));

        for iter_count in 0..MAX_CONSTRAINTS_TIMES {
            self.solve_point_constraints();

            self.solve_join_constraints();

            self.solve_contact_constraints(iter_count);
        }

        self.integrate_position(delta_time);

        dbg!(self
            .get_element(2)
            .map(|element| { element.meta().angle() }));

        dbg!(self
            .get_element(3)
            .map(|element| { element.meta().angle() }));

        dbg!("---------");

        if self.context.constraint_parameters.split_position_fix {
            for _ in 0..10 {
                self.solve_position_fix();
            }
        }

        dbg!(self
            .get_element(2)
            .map(|element| { element.meta().angle() }));

        dbg!(self
            .get_element(3)
            .map(|element| { element.meta().angle() }));

        self.contact_constraints
            .iter_mut()
            .for_each(|constraint| constraint.set_active(false));

        if !self.contact_constraints.is_empty() {
            // panic!("");
        }

        unsafe {
            // TODO update params
            self.reset_constraints_params(delta_time);
        }

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
    pub fn elements_iter(&self) -> impl Iterator<Item = &Element<T>> {
        self.element_store.iter()
    }

    #[inline]
    pub fn elements_iter_mut(&mut self) -> impl Iterator<Item = &mut Element<T>> {
        self.element_store.iter_mut()
    }

    #[inline]
    pub fn get_element(&self, id: ID) -> Option<&Element<T>> {
        self.element_store.get_element_by_id(id)
    }

    #[inline]
    pub fn get_element_mut(&mut self, id: ID) -> Option<&mut Element<T>> {
        self.element_store.get_mut_element_by_id(id)
    }

    #[inline]
    pub fn frame_count(&self) -> u128 {
        self.frame_count
    }

    #[inline]
    pub fn context_mut(&mut self) -> &mut Context {
        &mut self.context
    }

    // remove all elements;
    #[inline]
    pub fn clear(&mut self) {
        self.element_store.clear();
        self.id_dispatcher.reset();
        self.constraints_id_dispatcher.reset();
        self.contact_constraints.clear();
        self.point_constraints.clear();
        self.join_constraints.clear();
        self.frame_count = 0;
    }

    // TODO  use exist collision manifold
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

    pub fn set_gravity(&mut self, reducer: impl Fn(&Vector) -> Vector) {
        let context = &mut self.context;
        context.default_gravity = reducer(&context.default_gravity);
    }

    // TODO doc
    pub fn create_point_constraint(
        &mut self,
        element_id: ID,
        element_point: impl Into<Point>,
        fixed_point: impl Into<Point>,
        config: impl Into<JoinConstraintConfig>,
    ) -> Option<u32> {
        let config: JoinConstraintConfig = config.into();

        assert!(
            config.distance >= 0.,
            "distance must large than or equal to zero"
        );

        let id = self.constraints_id_dispatcher.gen_id();

        let element = self.get_element_mut(element_id)?;

        let element_point = element_point.into();
        let fixed_point = fixed_point.into();

        element.create_bind_point(id, element_point);

        let point_constraint =
            PointConstraint::new(id, element_id, fixed_point, element_point, config);

        self.point_constraints.insert(id, point_constraint);

        id.into()
    }

    pub fn point_constraints(&self) -> impl Iterator<Item = &PointConstraint<Element<T>>> {
        self.point_constraints.values()
    }

    pub fn get_point_constraint(&self, id: u32) -> Option<&PointConstraint<Element<T>>> {
        self.point_constraints.get(&id)
    }

    pub fn get_point_constraint_mut(
        &mut self,
        id: u32,
    ) -> Option<&mut PointConstraint<Element<T>>> {
        self.point_constraints.get_mut(&id)
    }

    pub fn remove_point_constraint(&mut self, id: u32) -> Option<PointConstraint<Element<T>>> {
        self.point_constraints.remove(&id).map(|point_constraint| {
            if let Some(element) = self.get_element_mut(point_constraint.obj_id()) {
                element.remove_bind_point(point_constraint.id())
            }
            point_constraint
        })
    }

    pub fn create_join_constraint(
        &mut self,
        element_a_id: ID,
        element_a_point: impl Into<Point>,
        element_b_id: ID,
        element_b_point: impl Into<Point>,
        config: impl Into<JoinConstraintConfig>,
    ) -> Option<u32> {
        let config: JoinConstraintConfig = config.into();

        assert!(
            config.distance >= 0.,
            "distance must large than or equal to zero"
        );

        let id = self.constraints_id_dispatcher.gen_id();
        if element_a_id == element_b_id {
            // TODO result
            panic!("can't be the same id");
        }

        let (element_a, element_b) = self.query_element_pair_mut((element_a_id, element_b_id))?;

        let element_a_point = element_a_point.into();
        let element_b_point = element_b_point.into();

        element_a.create_bind_point(id, element_a_point);
        element_b.create_bind_point(id, element_b_point);

        let join_constraint = JoinConstraint::new(
            id,
            (element_a_id, element_b_id),
            (element_a_point, element_b_point),
            config,
        );

        self.join_constraints.insert(id, join_constraint);

        id.into()
    }

    pub fn join_constraints(&self) -> impl Iterator<Item = &JoinConstraint<Element<T>>> {
        self.join_constraints.values()
    }

    pub fn get_join_constraint(&self, id: u32) -> Option<&JoinConstraint<Element<T>>> {
        self.join_constraints.get(&id)
    }

    pub fn get_join_constraint_mut(&mut self, id: u32) -> Option<&mut JoinConstraint<Element<T>>> {
        self.join_constraints.get_mut(&id)
    }

    pub fn remove_join_constraint(&mut self, id: u32) -> Option<JoinConstraint<Element<T>>> {
        self.join_constraints.remove(&id).map(|join_constraint| {
            if let Some((element_a, element_b)) =
                self.query_element_pair_mut(join_constraint.obj_id_pair())
            {
                element_a.remove_bind_point(join_constraint.id());
                element_b.remove_bind_point(join_constraint.id());
            }
            join_constraint
        })
    }

    // clear velocity for  all element , just set zero to velocity
    pub fn silent(&mut self) {
        self.elements_iter_mut()
            .map(|element| element.meta_mut())
            .for_each(|meta| {
                meta.set_angle_velocity(|_| 0.);
                meta.set_velocity(|_| Default::default());
            })
    }

    fn integrate_velocity(&mut self, delta_time: FloatNum) {
        let gravity = self.context.default_gravity;
        let enable_gravity = self.context.enable_gravity;
        self.elements_iter_mut()
            .filter(|element| !element.meta().is_fixed())
            .filter(|element| !element.meta().is_sleeping())
            .filter(|element| !element.meta().is_ignore_gravity())
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

    unsafe fn reset_constraints_params(&mut self, delta_time: FloatNum) {
        let mut legacy_constraint_ids = vec![];
        let self_ptr = self as *mut Scene<_>;

        self.elements_iter_mut().for_each(|element| {
            *element.meta_mut().contact_count_mut() = 0;
        });

        for contact_constraint in (*self_ptr).contact_constraints.iter_mut() {
            let Some((element_a, element_b)) =
                (*self_ptr).query_element_pair_mut(contact_constraint.obj_id_pair())
            else {
                // legacy_constraint_ids.push(contact_constraint.id());
                continue;
            };

            let obj_a = element_a as *mut _;
            let obj_b = element_b as *mut _;

            contact_constraint.reset_params(
                (obj_a, obj_b),
                delta_time,
                &self.context.constraint_parameters,
            )
        }

        for point_constraint in (*self_ptr).point_constraints.values_mut() {
            let Some(element) = (*self_ptr).get_element_mut(point_constraint.obj_id()) else {
                legacy_constraint_ids.push(point_constraint.id());
                continue;
            };

            let Some(move_point) = element.get_bind_point(point_constraint.id()) else {
                legacy_constraint_ids.push(point_constraint.id());
                continue;
            };

            let move_point = *move_point;
            let obj = element as *mut _;
            point_constraint.reset_params(move_point, obj, delta_time);
        }

        legacy_constraint_ids.iter().for_each(|id| {
            self.point_constraints.remove(id);
        });

        legacy_constraint_ids.truncate(0);

        for join_constraint in (*self_ptr).join_constraints.values_mut() {
            let Some((element_a, element_b)) =
                (*self_ptr).query_element_pair_mut(join_constraint.obj_id_pair())
            else {
                legacy_constraint_ids.push(join_constraint.id());
                continue;
            };

            let obj_a = element_a as *mut _;
            let obj_b = element_b as *mut _;

            let Some(move_point_a) = (*element_a).get_bind_point(join_constraint.id()) else {
                legacy_constraint_ids.push(join_constraint.id());
                continue;
            };
            let Some(move_point_b) = (*element_b).get_bind_point(join_constraint.id()) else {
                legacy_constraint_ids.push(join_constraint.id());
                continue;
            };

            join_constraint.reset_params(
                (obj_a, obj_b),
                (*move_point_a, *move_point_b),
                delta_time,
            );
        }

        legacy_constraint_ids.iter().for_each(|id| {
            self.join_constraints.remove(id);
        });
    }

    fn solve_point_constraints(&mut self) {
        self.point_constraints
            .values_mut()
            .for_each(|constraint| unsafe { constraint.solve(&self.context.constraint_parameters) })
    }

    fn solve_join_constraints(&mut self) {
        self.join_constraints
            .values_mut()
            .for_each(|join_constraint| unsafe {
                join_constraint.solve(&self.context.constraint_parameters)
            })
    }

    fn solve_contact_constraints(&mut self, iter_count: u8) {
        self.contact_constraints
            .iter_mut()
            .filter(|constraint| constraint.is_active())
            .for_each(|contact_constraint| unsafe {
                contact_constraint
                    .solve_velocity_constraint(&self.context.constraint_parameters, iter_count);
            })
    }

    // separate contact object by change their position directly;
    fn solve_position_fix(&mut self) {
        self.contact_constraints
            .iter_mut()
            .filter(|constraint| constraint.is_active())
            .enumerate()
            .for_each(|(index, contact_constraint)| unsafe {
                contact_constraint
                    .solve_position_constraint(&self.context.constraint_parameters, index);
            })
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
    ) -> Option<(&Element<T>, &Element<T>)> {
        if element_a_id == element_b_id {
            return None;
        }

        let element_a = self.element_store.get_element_by_id(element_a_id)?;

        let element_b = self.element_store.get_element_by_id(element_b_id)?;

        (element_a, element_b).into()
    }

    fn query_element_pair_mut(
        &mut self,
        (element_a_id, element_b_id): (ID, ID),
    ) -> Option<(&mut Element<T>, &mut Element<T>)> {
        if element_a_id == element_b_id {
            return None;
        }

        let element_a = self.get_element_mut(element_a_id)? as *mut Element<_>;

        let element_b = self.get_element_mut(element_b_id)? as *mut Element<_>;
        unsafe { (&mut *element_a, &mut *element_b).into() }
    }
}

impl<T, Z> Shl<T> for &mut Scene<Z>
where
    Z: Clone + Default,
    T: Into<Element<Z>>,
{
    type Output = ID;
    fn shl(self, rhs: T) -> Self::Output {
        self.push_element(rhs.into())
    }
}
